# Consul-Monitor

Nagios/Centreon-compatible check commands for Consul cluster and services


# Common arguments

Arguments available with all checks :

* --host CONSUL_HOST : default to 127.0.0.1
* --port CONSUL_PORT : default to 8500
* --token CONSUL_TOKEN : default to None, Consul token used to query API (service-health and node-service-health)
* --verbose : shows verbose output on error
* --critical-on-error : returns CRITICAL instead of UNKNOWN if an error occurred while checking

The minimum and maximum thresholds can be set to -1 to be disabled.

# Check leader

Check Consul cluster leader, critical if not LEADER_IP:PORT.

```shell
consul-monitor --mode leader [--expected-leader LEADER_IP]
```

# Check cluster peers

Check Consul cluster peers count, critical if different than COUNT

```shell
consul-monitor --mode peers [--expected-peer-count COUNT]
```

# Check service instance count

Check how many service instances have passing status in Consul.

```shell
consul-monitor --mode service-health --service SERVICE_NAME [--warning-min MIN] [--critical-min MIN] [--warning-max MAX] [--critical-max MAX] [--tag TAG] [--token CONSUL_TOKEN]
```

# Check service/check on a node

Check how many service/check have passing status in Consul on a node.

```shell
consul-monitor --mode node-service-health --node NODE [--service SERVICE_NAME|--check-id CHECK_ID] [--warning-min MIN] [--critical-min MIN] [--warning-max MAX] [--critical-max MAX] [--token CONSUL_TOKEN]
```
