extern crate consul;
extern crate argparse;

use std::process::exit;
use consul::{Client, Config};
use consul::health::{Health, ServiceEntry};
use argparse::{ArgumentParser, StoreOption, Store};
use consul::status::Status;

fn print_service_perfdata_and_details(passing_services_count: &u32, srvcs: &Vec<ServiceEntry>) {
    println!("|instance_count={}", passing_services_count);
    for srvc in srvcs {
        println!("{} on node {}", srvc.Service.Service, srvc.Node.Node);
        for c in &srvc.Checks {
            println!("\tCheck '{}' is {} : {}", c.CheckID, c.Status, c.Output);
        }
    }
}

fn check_service(c: Client, wmin: Option<u32>, wmax: Option<u32>, cmin: Option<u32>, cmax: Option<u32>, service_name: Option<String>) {
    let s_service_name = service_name.unwrap();
    let consul_srvc_res = c
        .service(&*s_service_name, None, false, None)
        .expect(&*format!("Failed to get service {}", s_service_name));

    let mut passing_services_count = 0;
    for srvc in &consul_srvc_res.0 {
        let mut all_checks_passing = true;
        for c in &srvc.Checks {
            if c.Status != "passing".to_string() {
                all_checks_passing = false;
            }
        }
        if all_checks_passing {
            passing_services_count += 1;
        }
    }

    let res;
    if cmin.is_some() && passing_services_count <= cmin.unwrap() {
        print!("CRITICAL : Not enough {} service instances", s_service_name);
        res = 2;
    } else if wmin.is_some() && passing_services_count <= wmin.unwrap() {
        print!("WARNING : Not enough {} service instances", s_service_name);
        res = 1;
    } else if cmax.is_some() && passing_services_count >= cmax.unwrap() {
        print!("CRITICAL : Too many {} service instances", s_service_name);
        res = 2;
    } else if wmax.is_some() && passing_services_count >= wmax.unwrap() {
        print!("WARNING : Too many {} service instances", s_service_name);
        res = 1;
    } else {
        println!("OK : {} passing {} service instances", passing_services_count, s_service_name);
        res = 0;
    }
    print_service_perfdata_and_details(&passing_services_count, consul_srvc_res.0.as_ref());
    exit(res)
}

fn check_leader(c: Client, expected_leader: Option<String>) {
    let leader_res = c.leader(None);
    if let Ok(leader) = leader_res {
        if let Some(el) = expected_leader {
            if el != leader.0 {
                println!("{} is not the expected cluster leader (expected {})", el, leader.0);
                exit(2);
            }
        }
        println!("Cluster leader is {}", leader.0);
        exit(0);
    } else {
        println!("Failed to get leader");
        exit(3);
    }
}

fn print_peers_perfdata_and_details(peers: &Vec<String>) {
    println!("|peers={}", peers.len());
    for peer in peers {
        println!("{}", peer);
    }
}

fn check_peers(c: Client, expected_peer_count: Option<usize>) {
    let peers_res = c.peers(None);
    if let Ok(peers) = peers_res {
        if let Some(epc) = expected_peer_count {
            if epc != peers.0.len() {
                print!("Expected {} peers in cluster, found {}", epc, peers.0.len());
                print_peers_perfdata_and_details(&peers.0);
                exit(2);
            }
        }
        print!("{} peers in cluster", peers.0.len());
        print_peers_perfdata_and_details(&peers.0);
        exit(0);
    } else {
        println!("Failed to get leader");
        exit(3);
    }
}

fn main() {
    let mut mode = String::new();
    let mut host = String::new();
    let mut port: Option<u16> = None;
    let mut token: Option<String> = None;
    let mut warning_min: Option<u32> = None;
    let mut warning_max: Option<u32> = None;
    let mut critical_min: Option<u32> = None;
    let mut critical_max: Option<u32> = None;
    let mut service_name: Option<String> = None;
    let mut expected_leader: Option<String> = None;
    let mut expected_peer_count: Option<usize> = None;

    // consul_monitoring.exe --mode health --host 10.101.6.7 --token 31f08140-ed42-cdc6-086e-df7b52cfa4dc --service consul --critical-min 1 --warning-max 2

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Greet somebody.");
        ap.refer(&mut mode)
            .add_option(&["-m", "--mode"], Store,
                        "Consul check mode (leader, cluster, service, health)");
        ap.refer(&mut host)
            .add_option(&["-h", "--host"], Store,
                        "Consul service host");
        ap.refer(&mut port)
            .add_option(&["--port"], StoreOption,
                        "Consul HTTP API Port (default 8500)");
        ap.refer(&mut token)
            .add_option(&["--token"], StoreOption,
                        "Consul token");
        ap.refer(&mut warning_min)
            .add_option(&["--warning-min"], StoreOption,
                        "Warning if less than that value");
        ap.refer(&mut warning_max)
            .add_option(&["--warning-max"], StoreOption,
                        "Warning if more than that value");
        ap.refer(&mut critical_min)
            .add_option(&["--critical-min"], StoreOption,
                        "Critical if less than that value");
        ap.refer(&mut critical_max)
            .add_option(&["--critical-max"], StoreOption,
                        "Critical if more than that value");
        ap.refer(&mut service_name)
            .add_option(&["--service"], StoreOption,
                        "Service name");
        ap.refer(&mut expected_leader)
            .add_option(&["--expected-leader"], StoreOption,
                        "Expected cluster leader");
        ap.refer(&mut expected_peer_count)
            .add_option(&["--expected-peer-count"], StoreOption,
                        "Expected peer count in cluster");
        ap.parse_args_or_exit();
    }

    let config = Config::new_from_consul_host(format!("http://{}", host).as_ref(), port, token).expect("Impossible de générer la configuration");
    let client = Client::new(config);
    match &*mode {
        "service" => check_service(client, warning_min, warning_max, critical_min, critical_max, service_name),
        "leader" => check_leader(client, expected_leader),
        "peers" => check_peers(client, expected_peer_count),
        "health" => panic!("Not implemented"),
        "" => panic!("No check mode found"),
        _ => panic!("No check mode {}", mode)
    }
}
