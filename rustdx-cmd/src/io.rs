use crate::cmd::DayCmd;
use anyhow::{anyhow, Result};
use rustdx::file::{
    day::fq::Day,
    gbbq::{Factor, Gbbq},
};
use rustdx_cmd::fetch_code::StockList;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
    process::Command,
};

const BUFFER_SIZE: usize = 32 * (1 << 20); // 32M

/// TODO 协程解析、异步缓冲写入（利用多核优势）
pub fn run_csv(cmd: &DayCmd) -> Result<()> {
    let hm = cmd.stocklist();
    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    for dir in &cmd.path {
        let n = filter_file(dir)?.count();
        println!("dir: {dir:?} day 文件数量：{n}");
        let take = cmd.amount.unwrap_or(n);

        let mut count: usize = 0;
        filter_file(dir)?
            .map(|f| (cmd.filter_ec(f.to_str().unwrap()), f))
            .filter(|((b, _), s)| filter(*b, s, hm.as_ref(), dir).unwrap_or(false))
            .take(take)
            .filter_map(|((_, code), src)| {
                count += 1;
                println!("#{code:06}# {src:?}");
                rustdx::file::day::Day::from_file_into_vec(code, src).ok()
            })
            .flatten()
            .try_for_each(|t| wtr.serialize(t))?;

        print(dir, count, take);
    }
    wtr.flush().map_err(|e| e.into())
}

/// TODO 协程解析、异步缓冲写入（利用多核优势）
pub fn run_csv_fq(cmd: &DayCmd) -> Result<()> {
    // 股本变迁
    let mut bytes = fs::read(cmd.gbbq.as_ref().unwrap())?;
    let gbbq = Gbbq::filter_hashmap(Gbbq::iter(&mut bytes[4..]));

    // 股票列表
    let hm = cmd.stocklist();

    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    for dir in &cmd.path {
        let n = filter_file(dir)?.count();
        println!("dir: {dir:?} day 文件数量：{n}");
        let take = cmd.amount.unwrap_or(n);

        let mut count: usize = 0;
        filter_file(dir)?
            .map(|f| (cmd.filter_ec(f.to_str().unwrap()), f))
            .filter(|((b, _), s)| filter(*b, s, hm.as_ref(), dir).unwrap_or(false))
            .take(take)
            .filter_map(|((_, code), src)| {
                count += 1;
                println!("#{code:06}# {src:?}");
                Day::new(code, src, gbbq.get(&code).map(Vec::as_slice)).ok()
            })
            .flatten()
            .try_for_each(|t| wtr.serialize(t))?;

        print(dir, count, take);
    }
    wtr.flush().map_err(|e| e.into())
}

/// TODO 协程解析、异步缓冲写入（利用多核优势）
pub fn run_csv_fq_previous(cmd: &DayCmd) -> Result<()> {
    // 股本变迁
    let mut bytes = fs::read(cmd.gbbq.as_ref().unwrap())?;
    let gbbq = Gbbq::filter_hashmap(Gbbq::iter(&mut bytes[4..]));

    // 前收
    let previous = previous_csv_table(&cmd.previous, &cmd.table)?;

    // 股票列表
    let hm = cmd.stocklist();

    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    for dir in &cmd.path {
        let n = filter_file(dir)?.count();
        println!("dir: {dir:?} day 文件数量：{n}");
        let take = cmd.amount.unwrap_or(n);

        let mut count: usize = 0;
        filter_file(dir)?
            .map(|f| (cmd.filter_ec(f.to_str().unwrap()), f))
            .filter(|((b, _), s)| filter(*b, s, hm.as_ref(), dir).unwrap_or(false))
            .take(take)
            .filter_map(|((_, code), src)| {
                count += 1;
                println!("#{code:06}# {src:?}");
                Day::concat(
                    code,
                    src,
                    // 无分红数据并不意味着无复权数据
                    gbbq.get(&code).map(Vec::as_slice),
                    previous.get(&code),
                )
                .ok()
            })
            .flatten()
            .try_for_each(|t| wtr.serialize(t))?;

        print(dir, count, take);
    }
    wtr.flush().map_err(|e| e.into())
}

/// 筛选 day 文件
#[rustfmt::skip]
fn filter_file(dir: &Path) -> Result<impl Iterator<Item = std::path:: PathBuf>> {
    Ok(dir.read_dir()?
          .filter_map(|e| e.map(|f| f.path()).ok())
          .filter(|p| p.extension().map(|s| s == "day").unwrap_or_default()))
}

/// 筛选存在于股票列表的股票
#[inline]
fn filter(b: bool, src: &Path, hm: Option<&StockList>, dir: &Path) -> Option<bool> {
    let src = src.strip_prefix(dir).ok()?.to_str()?.strip_suffix(".day")?;
    Some(b && hm.map(|m| m.contains(src)).unwrap_or(true))
}

fn print(dir: &Path, count: usize, take: usize) {
    if count == 0 && take != 0 {
        println!("{dir:?} 目录下无 `.day` 文件符合要求");
    } else if take == 0 {
        println!("请输入大于 0 的文件数量");
    } else {
        println!("{dir:?}\t已完成：{count}");
    }
}

fn database_table(table: &str) -> (&str, &str) {
    let pos = table.find('.').unwrap();
    table.split_at(pos) // (database_name, table_name)
}

pub fn setup_clickhouse(fq: bool, table: &str) -> Result<()> {
    let create_database = format!("CREATE DATABASE IF NOT EXISTS {}", database_table(table).0);
    let output = Command::new("clickhouse-client")
        .args(["--query", &create_database])
        .output()?;
    check_output(output);
    #[rustfmt::skip]
    let create_table = if fq {
        format!("
            CREATE TABLE IF NOT EXISTS {table}
            (
                `date` Date CODEC(DoubleDelta),
                `code` FixedString(6),
                `open` Float32,
                `high` Float32,
                `low` Float32,
                `close` Float32,
                `amount` Float64,
                `vol` Float64,
                `preclose` Float64,
                `factor` Float64
            )
            ENGINE = ReplacingMergeTree()
            ORDER BY (date, code)
        ")
    } else {
        format!("
            CREATE TABLE IF NOT EXISTS {table}
            (
                `date` Date CODEC(DoubleDelta),
                `code` FixedString(6),
                `open` Float32,
                `high` Float32,
                `low` Float32,
                `close` Float32,
                `amount` Float64,
                `vol` Float64
            )
            ENGINE = ReplacingMergeTree()
            ORDER BY (date, code)
        ")
    }; // PARTITION BY 部分可能需要去掉
    let output = Command::new("clickhouse-client")
        .args(["--query", &create_table])
        .output()?;
    check_output(output);
    Ok(())
}

pub fn insert_clickhouse(output: &impl AsRef<Path>, table: &str, keep: bool) -> Result<()> {
    use subprocess::{Exec, Redirection};
    let query = format!("INSERT INTO {table} FORMAT CSVWithNames");
    let capture = Exec::cmd("clickhouse-client")
        .args(&["--query", &query])
        .stdin(Redirection::File(File::open(output)?))
        .capture()?;
    println!(
        "stdout:\n{}stderr:\n{}",
        capture.stdout_str(),
        capture.stderr_str()
    );
    assert!(capture.success());
    keep_csv(output, keep)?;
    Ok(())
}

/// 需要日线 clickhouse csv 文件
#[test]
fn test_insert_clickhouse() -> Result<()> {
    setup_clickhouse(true, "rustdx.tmp")?;
    insert_clickhouse(&"clickhouse", "rustdx.tmp", true)
}

type Previous = Result<std::collections::HashMap<u32, Factor>>;

pub fn previous_csv_table(path: &Option<std::path::PathBuf>, table: &str) -> Previous {
    if let Some(Some(path)) = path.as_ref().map(|p| p.to_str()) {
        if path == "clickhouse" {
            clickhouse_factor_csv(table)
        } else {
            previous_csv(path)
        }
    } else {
        Err(anyhow!("请检查 gbbq 路径"))
    }
}

/// 读取前收盘价（前 factor ）数据
pub fn previous_csv(p: impl AsRef<Path>) -> Previous {
    let path = p.as_ref();
    let prev = csv::Reader::from_reader(File::open(path)?)
        .deserialize::<Factor>()
        .filter_map(|f| f.ok())
        .map(|f| (f.code.parse().unwrap(), f))
        .collect();
    fs::remove_file(path)?;
    Ok(prev)
}

/// 获取当前最新 factor
fn clickhouse_factor_csv(table: &str) -> Previous {
    let args = [
        "--query",
        &format!(
            "SELECT
                     yesterday() AS date,
                     code,
                     last_value(close) AS close,
                     last_value(factor) AS factor
                    FROM {table}
                    GROUP BY code
                    INTO OUTFILE 'factor.csv'
                    FORMAT CSVWithNames;"
        ),
    ];
    let output = Command::new("clickhouse-client").args(args).output()?;
    check_output(output);
    previous_csv("factor.csv")
}

/// TODO: 与数据库有关的，把库名、表名可配置
pub fn run_mongodb(cmd: &DayCmd) -> Result<()> {
    cmd.run_csv()?;
    // TODO:排查为什么 date 列无法变成 date 类型 date.date(2006-01-02)
    let (database_name, table_name) = database_table(&cmd.table);
    let args = [
        "--db",
        database_name,
        "--collection",
        table_name,
        "--type=csv",
        "--file",
        &cmd.output,
        "--columnsHaveTypes",
        "--fields=code.string()",
    ];
    let output = Command::new("mongoimport").args(args).output()?;
    check_output(output);
    keep_csv(&cmd.output, cmd.keep_csv)?;
    Ok(())
}

fn check_output(output: std::process::Output) {
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    assert!(output.status.success());
}

fn keep_csv(fname: &impl AsRef<Path>, keep: bool) -> io::Result<()> {
    if keep {
        fs::rename(fname, fname.as_ref().with_extension("csv"))
    } else {
        fs::remove_file(fname)
    }
}

/// 读取本地 xls(x) 文件
pub fn read_xlsx(path: &str, col: usize, prefix: &str) -> Option<StockList> {
    use calamine::{open_workbook_auto, DataType, Reader};
    let mut workbook = open_workbook_auto(path).ok()?;
    let format_ = |x: &str| format!("{}{}", crate::cmd::auto_prefix(prefix, x), x);
    // 每个单元格被解析的类型可能会不一样，所以把股票代码统一转化成字符型
    if let Some(Ok(range)) = workbook.worksheet_range_at(0) {
        Some(
            range
                .rows()
                .skip(1)
                .map(|r| match &r[col] {
                    DataType::Int(x) => format_(&x.to_string()),
                    DataType::Float(x) => format_(&(*x as i64).to_string()),
                    DataType::String(x) => format_(x),
                    _ => unreachable!(),
                })
                .collect(),
        )
    } else {
        None
    }
}
