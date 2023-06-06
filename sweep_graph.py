from influxdb_client import InfluxDBClient
import datetime
import matplotlib.pyplot as plt

token = "nKiG9TvFBHvyi2J0_ge9JjZTu-322ermASXzErFwJDJ7hxgnX33l7701Hdxs_bX2NZKcxwWJ1mYbSHD6ozWMaA=="
org = "angstromlab"
ip = "192.168.3.140"

client = InfluxDBClient(url=f'{ip}:8086', token=token, org=org)

query_client = client.query_api()

query = 'from(bucket:"rooftop")\
|> range(start: -24h)\
|> filter(fn:(r) => r._measurement == "sweep")'

res = query_client.query(query=query)

hosts = {}

for data_point in res:
    for record in data_point.records:
        host = record.values["host"]
        if not host in hosts:
            hosts[host] = []
        hosts[host].append(record)

for host in hosts:
    max_time = max([x.get_time() for x in hosts[host]])
    min_time = max_time - datetime.timedelta(seconds=1)

    voltages = []
    currents = []
    for record in hosts[host]:
        if record.get_time() < min_time:
            continue
        if record.get_field() == "voltage":
            voltages.append((record.get_time(), record.get_value()))
        elif record.get_field() == "current":
            currents.append((record.get_time(), record.get_value()))

    x = []
    y_current = []
    y_power = []

    for voltage in voltages:
        for current in currents:
            if voltage[0] == current[0]:
                x.append(voltage[1])
                y_current.append(current[1])
                y_power.append(current[1] * voltage[1])

    plt.figure()
    plt.plot(x, y_current)
    plt.xlabel("Voltage (V)")
    plt.ylabel("Current (A)")
    plt.figure()
    plt.plot(x, y_power)
    plt.xlabel("Voltage (V)")
    plt.ylabel("Power (W)")
    
plt.show()