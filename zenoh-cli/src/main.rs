
mod action;
mod parser;


fn set_required_options(config: &mut zenoh::config::Config) {
    config.insert_json5("plugins_loading/enabled", "true").unwrap();
    config.insert_json5("plugins/storage_manager/__required__","true").unwrap();
    config.insert_json5("metadata", r#"{ name: "Zenoh Swiss Army Knife", location: "My Laptop" }"#).unwrap();
    config.insert_json5("timestamping", r#"{ enabled: { router: true, peer: true, client: true }, drop_future_timestamp: false }"#).unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = parser::arg_parser().get_matches();


    let mut config = match matches.get_one::<String>("config") {
        Some(fname) => zenoh::Config::from_file(fname).expect("Unable to open the Zenoh Config"),
        None => {
            zenoh::Config::default()
        }
    };

    match matches.get_one::<String>("mode") {
        Some(m) => {
            match m.as_str() {
                "peer" => {},
                "client" => {},
                "router" => {},
                _ => {
                    println!("Invalid mode: {}", m);
                    return;
                }

            }
            config.insert_json5("mode", &format!("\"{}\"",m)).unwrap();
        },
        None => {}
    };

    set_required_options(&mut config);

    match matches.get_one::<bool>("disable_scouting") {
        Some(disabled) => {
            if *disabled {
                config.insert_json5("scouting/multicast/enabled", "false").unwrap();
            }
        }, None => {}
    };

    match matches.get_one::<String>("endpoints") {
        Some(es) => {
            config.insert_json5("connect/endpoints", es).unwrap();
        },
        None => {}
    };



    match matches.get_one::<String>("rest") {
        Some(port) => {
            config.insert_json5(
                "plugins/rest",
                &format!("{{http_port: {} }}", port)).unwrap();
        },
        None => {}
    }
    match matches.get_one::<bool>("admin") {
        Some(_) => {
            config.insert_json5("adminspace/enabled", "true").unwrap();
            config.insert_json5("adminspace/permissions", "{ read: true, write: true }").unwrap();
        },
        None => {}
    }

    let z = zenoh::open(config.clone())
        .await
        .expect("Unable to open the Zenoh Session");

    let wait_for_ctrl_c = match matches.subcommand() {
        Some(("scout", sub_matches)) => {
            action::do_scout(&z, sub_matches).await;
            false
        }
        Some(("publish", sub_matches)) => {
            action::do_publish(&z, sub_matches).await;
            false
        }
        Some(("subscribe", sub_matches)) => {
            println!("Ctrl-C to quit");
            action::do_subscribe(&z, sub_matches).await;
            false
        }
        Some(("query", sub_matches)) => {
            action::do_query(&z, sub_matches).await;
            false
        },
        Some(("queryable", sub_matches)) => {
            println!("Ctrl-C to quit");
            action::do_queryable(&z, sub_matches).await;
            false
        },
        Some(("storage", sub_matches)) => {
            let zid = z.zid().to_string();
            let zask = format!("@/{}/peer/config/plugins/storage_manager/storages/zenoh-storage", &zid);
            let kexpr = sub_matches.get_one::<String>("KEY_EXPR").unwrap();
            let replication = if sub_matches.get_one::<bool>("align").is_some() {
                r#", replication: { interval: 3, sub_intervals: 5, hot: 6, warm: 24, propagation_delay: 10}"#
            } else { "" };
            let storage_cfg = format!("{{ key_expr: \"{}\", volume: \"memory\" {} }}", kexpr, replication);
            z.put(zask, storage_cfg).await.unwrap();
            true
        },
        _ => { false }
    };
    if wait_for_ctrl_c {
        println!("Ctrl-C to quit");
        tokio::signal::ctrl_c().await.unwrap();
    }

}
