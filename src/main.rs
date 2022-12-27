extern crate consul;
extern crate argparse;

use std::process::exit;
use consul::{Client, Config};
use consul::health::{Health, HealthCheck, ServiceEntry};
use argparse::{ArgumentParser, StoreOption, Store, StoreTrue};
use consul::errors::Error;
use consul::status::Status;

const EXIT_OK: i32 = 0;
const EXIT_WARNING: i32 = 1;
const EXIT_CRITICAL: i32 = 2;
const EXIT_UNKNOWN: i32 = 3;
static mut CRITICAL_ON_ERROR: bool = false;
static mut VERBOSE: bool = false;

fn error_exit(err_desc: &str, err: Error) -> ! {
    eprintln!("{}", err_desc);
    unsafe {
        if VERBOSE {
            let e = &err.1;
            eprintln!("{:?}", e);
        }
        exit(if CRITICAL_ON_ERROR {
            EXIT_CRITICAL
        } else {
            EXIT_UNKNOWN
        });
    }
}

fn print_service_perfdata_and_details(passing_service_count: &u32, srvcs: &Vec<ServiceEntry>) {
    println!("|instance_count={}", passing_service_count);
    for srvc in srvcs {
        println!("{} on node {} (tags: {})", srvc.Service.Service, srvc.Node.Node, srvc.Service.Tags.as_ref().map_or("".to_string(), |tags| { tags.join(", ") }));
        for c in &srvc.Checks {
            println!("\tCheck '{}' is {} : {}", c.CheckID, c.Status, c.Output);
        }
    }
}

fn check_service_health(c: Client, service_name: &str, wmin: Option<u32>, wmax: Option<u32>, cmin: Option<u32>, cmax: Option<u32>, tag: Option<String>) {
    match c.service(service_name, tag.as_deref(), false, None) {
        Ok(consul_srvc_res) => {
            let mut passing_service_count = 0;
            for srvc in &consul_srvc_res.0 {
                let mut all_checks_passing = true;
                for c in &srvc.Checks {
                    if c.Status != "passing".to_string() {
                        all_checks_passing = false;
                    }
                }
                if all_checks_passing {
                    passing_service_count += 1;
                }
            }

            let res;
            if cmin.is_some() && passing_service_count <= cmin.unwrap() {
                print!("CRITICAL : Not enough {} service instances", service_name);
                res = EXIT_CRITICAL;
            } else if wmin.is_some() && passing_service_count <= wmin.unwrap() {
                print!("WARNING : Not enough {} service instances", service_name);
                res = EXIT_WARNING;
            } else if cmax.is_some() && passing_service_count >= cmax.unwrap() {
                print!("CRITICAL : Too many {} service instances", service_name);
                res = EXIT_CRITICAL;
            } else if wmax.is_some() && passing_service_count >= wmax.unwrap() {
                print!("WARNING : Too many {} service instances", service_name);
                res = EXIT_WARNING;
            } else {
                print!("OK : {} passing {} service instances", passing_service_count, service_name);
                res = EXIT_OK;
            }
            print_service_perfdata_and_details(&passing_service_count, consul_srvc_res.0.as_ref());
            exit(res)
        },
        Err(err) => {
            error_exit("Failed to get service instances", err);
        }
    }
}

fn print_health_checks_perfdata_and_details(service_name: Option<String>, check_id: Option<String>, passing_check_count: u32, health_checks: &Vec<HealthCheck>) {
    println!("|passing_check_count={}", passing_check_count);
    let service_name = service_name.unwrap_or("None".to_owned());
    let check_id = check_id.unwrap_or("None".to_owned());
    for h in health_checks {
        println!("Check '{}' is {} : {}", h.CheckID, h.Status, h.Output);
    }
    println!();
    println!("(Filtered ServiceName : {service_name}, CheckID : {check_id})");
}

fn check_node_service_health(c: Client, node: &str, wmin: Option<u32>, wmax: Option<u32>, cmin: Option<u32>, cmax: Option<u32>, service: Option<String>, check_id: Option<String>) {
    if service.is_none() && check_id.is_none() {
        eprintln!("service or check-id must be provided for this check");
        exit(EXIT_UNKNOWN);
    }

    match c.node(node, check_id.as_deref(), service.as_deref(), None) {
        Ok(srvc_health) => {
            let mut passing_check_count = 0;
            for health in &srvc_health.0 {
                if health.Status == "passing".to_string() {
                    passing_check_count += 1;
                }
            }

            let res;
            if cmin.is_some() && passing_check_count <= cmin.unwrap() {
                print!("CRITICAL : Not enough passing checks");
                res = EXIT_CRITICAL;
            } else if wmin.is_some() && passing_check_count <= wmin.unwrap() {
                print!("WARNING : Not enough passing checks");
                res = EXIT_WARNING;
            } else if cmax.is_some() && passing_check_count >= cmax.unwrap() {
                print!("CRITICAL : Too many passing checks");
                res = EXIT_CRITICAL;
            } else if wmax.is_some() && passing_check_count >= wmax.unwrap() {
                print!("WARNING : Too many passing checks");
                res = EXIT_WARNING;
            } else {
                print!("OK : {} passing checks", passing_check_count);
                res = EXIT_OK;
            }
            print_health_checks_perfdata_and_details(service, check_id, passing_check_count, &srvc_health.0);
            exit(res)
        },
        Err(err) => {
            error_exit(&*format!("Failed to get service health on node {}", node), err);
        }
    }

}

fn check_leader(c: Client, expected_leader: Option<String>) {
    match c.leader(None) {
        Ok(leader) => {
            if let Some(el) = expected_leader {
                if el != leader.0 {
                    println!("{} is not the expected cluster leader (expected {})", el, leader.0);
                    exit(EXIT_CRITICAL);
                }
            }
            println!("Cluster leader is {}", leader.0);
            exit(EXIT_OK);
        },
        Err(e) => {
            error_exit("Failed to get leader", e);
        }
    };
}

fn print_peers_perfdata_and_details(peers: &Vec<String>) {
    println!("|peers={}", peers.len());
    for peer in peers {
        println!("{}", peer);
    }
}

fn check_peers(c: Client, expected_peer_count: Option<usize>) {
    match c.peers(None) {
        Ok(peers) => {
            if let Some(epc) = expected_peer_count {
                if epc != peers.0.len() {
                    print!("Expected {} peers in cluster, found {}", epc, peers.0.len());
                    print_peers_perfdata_and_details(&peers.0);
                    exit(EXIT_CRITICAL);
                }
            }
            print!("{} peers in cluster", peers.0.len());
            print_peers_perfdata_and_details(&peers.0);
            exit(EXIT_OK);
        },
        Err(err) => {
            error_exit("Failed to get peers", err);
        }
    }
}

fn main() {

    let mut mode = String::new();
    let mut host: Option<String> = None;
    let mut port: Option<u16> = None;
    let mut token: Option<String> = None;
    let mut warning_min: Option<u32> = None;
    let mut warning_max: Option<u32> = None;
    let mut critical_min: Option<u32> = None;
    let mut critical_max: Option<u32> = None;
    let mut service: Option<String> = None;
    let mut tag: Option<String> = None;
    let mut check_id: Option<String> = None;
    let mut node: Option<String> = None;
    let mut expected_leader: Option<String> = None;
    let mut expected_peer_count: Option<usize> = None;
    let mut expected_version: Option<String> = None;

    unsafe {
        let mut ap = ArgumentParser::new();
        ap.set_description("Nagios/Centreon compatible Consul check commands.");
        ap.refer(&mut mode)
            .add_option(&["-m", "--mode"], Store,
                        "Consul check mode (leader, cluster, service-health, node-service-health)");
        ap.refer(&mut host)
            .add_option(&["-h", "--host"], StoreOption,
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
        ap.refer(&mut service)
            .add_option(&["--service"], StoreOption,
                        "Service name");
        ap.refer(&mut tag)
            .add_option(&["--tag"], StoreOption,
                        "Service tag");
        ap.refer(&mut node)
            .add_option(&["--node"], StoreOption,
                        "Node name");
        ap.refer(&mut check_id)
            .add_option(&["--check-id"], StoreOption,
                        "CheckID");
        ap.refer(&mut expected_leader)
            .add_option(&["--expected-leader"], StoreOption,
                        "Expected cluster leader");
        ap.refer(&mut expected_peer_count)
            .add_option(&["--expected-peers-count"], StoreOption,
                        "Expected peers count in cluster");
        ap.refer(&mut expected_version)
            .add_option(&["--expected-version"], StoreOption,
                        "Expected Consul service version");
        ap.refer(&mut CRITICAL_ON_ERROR)
            .add_option(&["--critical-on-error"], StoreTrue,
                        "Exit with critical status on error");
        ap.refer(&mut VERBOSE)
            .add_option(&["--verbose"], StoreTrue,
                        "Print extended output");
        ap.parse_args_or_exit();
    }

    let config = Config::new_from_consul_host(format!("http://{}", host.unwrap_or("127.0.0.1".to_owned())).as_ref(), port, token).expect("Impossible de générer la configuration");
    let client = Client::new(config);
    match &*mode {
        "service-health" => {
            let service = service.unwrap_or_else(|| {
                eprintln!("service must be provided for this mode");
                exit(EXIT_UNKNOWN);
            });
            check_service_health(client, &*service, warning_min, warning_max, critical_min, critical_max, tag)
        },
        "leader" => check_leader(client, expected_leader),
        "peers" => check_peers(client, expected_peer_count),
        "node-service-health" => {
            let node = node.unwrap_or_else(|| {
                eprintln!("node must be provided in this mode");
                exit(EXIT_UNKNOWN);
            });
            check_node_service_health(client, &*node, warning_min, warning_max, critical_min, critical_max, service, check_id)
        },
        "" => { eprintln!("No check mode found"); exit(EXIT_UNKNOWN) },
        _ => { eprintln!("Unknown check mode {mode}"); exit(EXIT_UNKNOWN) },
    }
}
