use actix_rt;

use qapro_rs::qaconnector::clickhouse::ckclient;
use qapro_rs::qaconnector::clickhouse::ckclient::DataConnector;
use qapro_rs::qadatastruct::stockday::QADataStruct_StockDay;
use qapro_rs::qaenv::localenv::CONFIG;

use polars::frame::DataFrame;
use polars::prelude::*;

use polars::series::ops::NullBehavior;
use std::fs::File;

extern crate stopwatch;

#[actix_rt::main]
async fn main() {
    ///
    /// this example is load cache from cachedir which defined in config.toml/example.toml
    ///
    ///
    /// cachedir/stockday.parquet
    ///
    let c = ckclient::QACKClient::init();

    //let stocklist = c.get_stocklist().await.unwrap();
    //let stocklist = c.get_stocklist().await.unwrap();

    let cache_file = format!("{}stockdayqfq.parquet", &CONFIG.DataPath.cache);
    let mut sw = stopwatch::Stopwatch::new();
    sw.start();
    let mut qfq = QADataStruct_StockDay::new_from_parquet(cache_file.as_str());
    println!("load cache 2year fullmarket stockdata {:#?}", sw.elapsed());
    println!("data  {:#?}", qfq.data.get_row(1).0);
    // println!("data  {:#?}", qfq.data.transpose());

    let cache_file = format!("{}stockadj.parquet", &CONFIG.DataPath.cache);

    // trait qatrans {
    //     fn transform_qadatastruct(data:DataFrame) -> Vec<QAKlineBase>;
    // }
    // impl  qatrans for DataFrame{
    //     fn transform_qadatastruct(data:DataFrame) -> Vec<QAKlineBase>{
    //         data.get_row(0)
    //     }
    // }

    // load factor
    let factor = c
        .get_factor("Asset_LR_Gr", "2019-01-01", "2021-12-25")
        .await
        .unwrap();

    sw.restart();
    let data_with_factor = qfq
        .data
        .join(
            &factor.data,
            &["date", "order_book_id"],
            &["date", "order_book_id"],
            JoinType::Inner,
            None,
        )
        .unwrap()
        .drop_duplicates(
            false,
            Some(&["date".to_string(), "order_book_id".to_string()]),
        )
        .unwrap();
    println!("join factor_data time {:#?}", sw.elapsed());
    println!("data_with_factor  {:#?}", data_with_factor);

    sw.restart();
    let rank = data_with_factor
        .groupby("date")
        .unwrap()
        .apply(|x| Ok(x.sort("factor", true).unwrap().head(Some(40))))
        .unwrap()
        .sort(&["date", "order_book_id"], false)
        .unwrap();

    println!("analysis factor_data time {:#?}", sw.elapsed());
    fn write_result(data: DataFrame, path: &str) {
        let file = File::create(path).expect("could not create file");

        ParquetWriter::new(file).finish(&data);
    }

    sw.restart();

    let rank4 = rank
        .sort("date", false)
        .unwrap()
        .lazy()
        .groupby([col("order_book_id")])
        .agg([
            col("close").pct_change(1).alias("pct"),
            col("date"),
            col("close"),
            col("open"),
            col("limit_up"),
            col("limit_down"),
            col("factor"),
        ])
        .select([
            col("order_book_id"),
            col("date"),
            col("close"),
            col("factor"),
            col("open"),
            col("limit_up"),
            col("limit_down"),
            col("pct"),
        ])
        .explode(vec![
            col("date"),
            col("close"),
            col("factor"),
            col("open"),
            col("limit_up"),
            col("limit_down"),
            col("pct"),
        ])
        .sort("date", false)
        .collect()
        .unwrap();

    println!("calc lazy time {:#?}", sw.elapsed());
    println!("lazy res {:#?}", rank4);
    pub fn get_row_vec(data: &DataFrame, idx: usize) -> Vec<AnyValue> {
        let values = data.iter().map(|s| s.get(idx)).collect::<Vec<_>>();
        values
    }
    //
    // pub fn get_row_vec(data:&DataFrame, idx: usize) -> Vec<(String, String,f32, f32, f32, f32, f32, f32)>{
    //     let values = data.iter().map(|s|
    //         match s.dtype(){
    //
    //
    //             DataType::Float32 => {s.f32().unwrap().get(idx).unwrap()}
    //
    //             DataType::Utf8 => {s.utf8().unwrap().get(idx).unwrap()}
    //             _ => {}
    //         }).collect::<(String, String,f32, f32, f32, f32, f32, f32)>();
    // }

    sw.restart();
    println!("res idx1 {:#?}", get_row_vec(&rank4, 1));
    println!("calc get row time {:#?}", sw.elapsed());

    sw.restart();
    // for i in 0..rank4.height() {
    //     let t = get_row_vec(&rank4, i);
    //     let code: String = t.get(0).unwrap().get(0).
    //     let datex: String = t.get(0).unwrap().1.clone();
    //     println!("{}-{}", code, datex);
    // }

    let closes = rank4["close"].f32().unwrap();
    let codes = rank4["order_book_id"].utf8().unwrap();
    let dates = rank4["date"].utf8().unwrap();
    for (code, (close, date)) in codes.into_iter().zip((closes.into_iter().zip(dates.into_iter()))) {

        let code2: &str =code.unwrap();
        let date2 :&str= date.unwrap();
        let close2 :f32 =close.unwrap();
    }

    println!("calc get row time {:#?}", sw.elapsed());

    //write_result(rank, "./cache/rankres.parquet");
}
