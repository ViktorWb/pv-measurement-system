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
|> aggregateWindow(every: 1ms, fn: mean, createEmpty: false)\
|> filter(fn:(r) => r._measurement == "mppt")'

res = query_client.query(query=query)

hosts = {}

for data_point in res:
    for record in data_point.records:
        host = record.values["host"]
        if not host in hosts:
            hosts[host] = []
        hosts[host].append(record)

for host in hosts:
    x_voltage = []
    y_voltage = []
    x_current = []
    y_current = []

    for record in hosts[host]:
        if record.get_field() == "voltage":
            x_voltage.append(record.get_time() + datetime.timedelta(hours = 2))
            y_voltage.append(record.get_value())
        elif record.get_field() == "current":
            x_current.append(record.get_time() + datetime.timedelta(hours = 2))
            y_current.append(record.get_value())

    plt.figure()
    plt.plot(x_voltage, y_voltage)
    plt.xlabel("Time")
    plt.ylabel("Voltage (V)")
    plt.figure()
    plt.plot(x_current, y_current)
    plt.xlabel("Time")
    plt.ylabel("Current (A)")
    
plt.show()