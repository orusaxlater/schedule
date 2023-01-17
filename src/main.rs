extern crate google_sheets4 as sheets4;
extern crate tokio;
use dotenvy::dotenv;
use scraper::{Html, Selector};
use sheets4::api::{ClearValuesRequest, ValueRange};
use sheets4::{hyper, hyper_rustls, oauth2, Error, Sheets};
use std::env;
use std::thread::sleep;
use std::time::Duration;

const INTERVAL: u64 = 1000 * 1;
const AXELIGHT_URL: &str = "https://axelight-official.com/schedule/index/num/";
const KOLOKOL_URL: &str = "https://kolokol-official.com/schedule/index/num/";
const QUUBI_URL: &str = "https://quubi.jp/schedule/index/num/";

struct Schedule {
    date: String,
    day_of_week: String,
    place: String,
    title: String,
    url: String,
}

struct Schedules {
    axelight: Vec<Schedule>,
    kolokol: Vec<Schedule>,
    quubi: Vec<Schedule>,
}

fn fetch_html(url: String) -> Result<String, Box<dyn std::error::Error>> {
    Ok(reqwest::blocking::get(url)?.text()?)
}

fn fetch_schedule_for_axelight() -> Vec<Schedule> {
    let html = fetch_html(String::from(AXELIGHT_URL) + "0").unwrap();
    let document = Html::parse_document(&html);
    let pager_selector = Selector::parse(".pagerSec > ul > li").unwrap();

    let mut pages = 0;
    let mut nums = vec![];

    // >が現れるまでをカウントしてページ数とする
    for element in document.select(&pager_selector) {
        let a = element.text().collect::<Vec<_>>();
        if a[0] == ">" {
            break;
        }

        nums.push((pages * 10).to_string());
        pages += 1;
    }
    if (nums.len() == 0) {
        nums.push("0".to_string())
    }

    let mut schedules: Vec<Schedule> = vec![];

    for num in nums {
        // 連続アクセスしないようにする。
        sleep(Duration::from_millis(INTERVAL));
        let url = String::from(AXELIGHT_URL) + &num;
        let html = fetch_html(url).unwrap();
        let document = Html::parse_document(&html);
        let schedule_selector = Selector::parse("article.scheduleList > section").unwrap();
        let elements = document.select(&schedule_selector);
        elements.for_each(|element| {
            let texts = element.text().collect::<Vec<_>>();
            let schedule = Schedule {
                date: texts[2].to_string(),
                day_of_week: texts[3].to_string(),
                place: texts[6].to_string(),
                title: texts[8].to_string(),
                url: String::from("https://axelight-official.com/schedule/detail/index/")
                    + &element.value().id().unwrap(),
            };
            schedules.push(schedule);
            println!("Date: {} {}", texts[2], texts[3]);
            println!("Place: {}", texts[6]);
            println!("Title: {}", texts[8]);
            println!("-------------------------------------------------------------------");
        })
    }

    return schedules;
}

fn fetch_schedule_for_kolokol() -> Vec<Schedule> {
    let html = fetch_html(String::from(KOLOKOL_URL) + "0").unwrap();
    let document = Html::parse_document(&html);
    let pager_selector = Selector::parse(".pagerSec > ul > li").unwrap();

    let mut pages = 0;
    let mut nums = vec![];

    // >が現れるまでをカウントしてページ数とする
    for element in document.select(&pager_selector) {
        let a = element.text().collect::<Vec<_>>();
        if a[0] == ">" {
            break;
        }

        nums.push((pages * 10).to_string());
        pages += 1;
    }
    if (nums.len() == 0) {
        nums.push("0".to_string())
    }

    let mut schedules: Vec<Schedule> = vec![];

    for num in nums {
        // 連続アクセスしないようにする。
        sleep(Duration::from_millis(INTERVAL));
        let url = String::from(KOLOKOL_URL) + &num;
        let html = fetch_html(url).unwrap();
        let document = Html::parse_document(&html);
        let selector = Selector::parse(".scdBox").unwrap();
        for element in document.select(&selector) {
            let texts = element.text().collect::<Vec<_>>();
            let schedule = Schedule {
                date: texts[3].to_string(),
                day_of_week: texts[4].to_string(),
                place: texts[16].to_string(),
                title: texts[14].to_string(),
                url: texts[28].to_string(),
            };
            println!("Date: {} {}", texts[3], texts[4]);
            println!("Place: {}", texts[16]);
            println!("Title: {}", texts[14]);
            println!("-------------------------------------------------------------------");

            schedules.push(schedule);
        }
    }

    return schedules;
}

fn fetch_schedule_for_quubi() -> Vec<Schedule> {
    let html = fetch_html(String::from(QUUBI_URL) + "0").unwrap();
    let document = Html::parse_document(&html);
    let pager_selector = Selector::parse(".pagerSec > ul > li").unwrap();

    let mut pages = 0;
    let mut nums = vec![];

    // >が現れるまでをカウントしてページ数とする
    for element in document.select(&pager_selector) {
        let a = element.text().collect::<Vec<_>>();
        if a[0] == ">" {
            break;
        }

        nums.push((pages * 10).to_string());
        pages += 1;
    }
    if (nums.len() == 0) {
        nums.push("0".to_string())
    }

    let mut schedules: Vec<Schedule> = vec![];

    for num in nums {
        // 連続アクセスしないようにする。
        sleep(Duration::from_millis(INTERVAL));
        let url = String::from(QUUBI_URL) + &num;
        let html = fetch_html(url).unwrap();
        let document = Html::parse_document(&html);
        let selector = Selector::parse(".record-list2 li").unwrap();
        for element in document.select(&selector) {
            let texts = element.text().collect::<Vec<_>>();
            let schedule = Schedule {
                date: texts[1].to_string(),
                day_of_week: texts[2].to_string(),
                place: texts[5].to_string(),
                title: texts[7].to_string(),
                url: String::from(""),
            };
            println!("Date: {}{}", texts[1], texts[2]);
            println!("Place: {}", texts[5]);
            println!("Title: {}", texts[7]);
            println!("-------------------------------------------------------------------");
            schedules.push(schedule);
        }
        let href_selector = Selector::parse(".record-list2 li > a").unwrap();
        let mut i = 0;
        for element in document.select(&href_selector) {
            let url = element.value().attr("href").unwrap();
            schedules[i].url = url.to_string();
            i += 1;
        }
    }

    return schedules;
}

#[tokio::main]
async fn write_schedule(schedules: Schedules, sheet_id: &str) {
    let secret = oauth2::read_application_secret("client_secret.json")
        .await
        .expect("not be read.");

    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokencache.json")
    .build()
    .await
    .unwrap();

    let hub = Sheets::new(
        hyper::Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build(),
        ),
        auth,
    );

    hub.spreadsheets()
        .values_clear(ClearValuesRequest::default(), sheet_id, "A3:L40")
        .doit()
        .await;

    let rows: Vec<_> = schedules
        .axelight
        .into_iter()
        .map(|s| {
            let date = String::from(s.date) + &s.day_of_week;
            vec![date, s.place, s.title, s.url]
        })
        .collect();
    let num = rows.len();

    let req = ValueRange {
        range: Some("".to_string()),
        major_dimension: Some("ROWS".to_string()),
        values: Some(rows),
    };

    let range = String::from("A3:D") + &(num + 2).to_string();

    let result = hub
        .spreadsheets()
        .values_update(req, sheet_id, &range)
        .value_input_option("RAW")
        .include_values_in_response(false)
        .response_value_render_option("FORMULA")
        .response_date_time_render_option("FORMATTED_STRING")
        .doit()
        .await;

    let rows2: Vec<_> = schedules
        .kolokol
        .into_iter()
        .map(|s| {
            let date = String::from(s.date) + &s.day_of_week;
            vec![date, s.place, s.title, s.url]
        })
        .collect();
    let num2 = rows2.len();

    let req2 = ValueRange {
        range: Some("".to_string()),
        major_dimension: Some("ROWS".to_string()),
        values: Some(rows2),
    };

    let range2 = String::from("E3:H") + &(num2 + 2).to_string();
    let result2 = hub
        .spreadsheets()
        .values_update(req2, sheet_id, &range2)
        .value_input_option("RAW")
        .include_values_in_response(false)
        .response_value_render_option("FORMULA")
        .response_date_time_render_option("FORMATTED_STRING")
        .doit()
        .await;

    let rows3: Vec<_> = schedules
        .quubi
        .into_iter()
        .map(|s| {
            let date = String::from(s.date) + &s.day_of_week;
            vec![date, s.place, s.title, s.url]
        })
        .collect();
    let num3 = rows3.len();

    let req3 = ValueRange {
        range: Some("".to_string()),
        major_dimension: Some("ROWS".to_string()),
        values: Some(rows3),
    };
    let range3 = String::from("I3:L") + &(num3 + 2).to_string();
    let result3 = hub
        .spreadsheets()
        .values_update(req3, sheet_id, &range3)
        .value_input_option("RAW")
        .include_values_in_response(false)
        .response_value_render_option("FORMULA")
        .response_date_time_render_option("FORMATTED_STRING")
        .doit()
        .await;

    match result {
        Err(e) => match e {
            Error::HttpError(_)
            | Error::Io(_)
            | Error::MissingAPIKey
            | Error::MissingToken(_)
            | Error::Cancelled
            | Error::UploadSizeLimitExceeded(_, _)
            | Error::Failure(_)
            | Error::BadRequest(_)
            | Error::FieldClash(_)
            | Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => println!("Success"),
    }
}

fn main() {
    dotenv().ok();
    let schedules = Schedules {
        axelight: fetch_schedule_for_axelight(),
        kolokol: fetch_schedule_for_kolokol(),
        quubi: fetch_schedule_for_quubi(),
    };

    write_schedule(schedules, &env::var("SHEET_ID").unwrap());
}