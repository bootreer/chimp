# Bird Migration Data
This sample data is adapted from the [Movebank: Animal Tracking](https://www.kaggle.com/pulkit8595/movebank-animal-tracking)
dataset available on [kaggle.com](https://kaggle.com).
Timestamps have been modified to take place in 2019.
The dataset as a whole represents animal migratory movements throughout 2019.
The data schema meets the requirements of the
[Flux Geo package](https://v2.docs.influxdata.com/v2.0/reference/flux/stdlib/experimental/geo/#geo-schema-requirements)
which contains tools for working with geo-temporal data.

## Use Sample Bird Migration Data
There are a few ways to use this sample data with InfluxDB:

### Load the CSV data directly in a Flux Query
Use the experimental `csv.from()` function to load annotated CSV data from GitHub:

```js
import "experimental/csv"

csv.from(url: "https://raw.githubusercontent.com/influxdata/influxdb2-sample-data/master/bird-migration-data/bird-migration.csv")
```

_**Note:** This downloads all the sample data each time you execute the query (~1.3 MB).
If bandwidth is a concern, use the [`to()` function](https://v2.docs.influxdata.com/v2.0/reference/flux/stdlib/built-in/outputs/to/)
to write the data to a bucket, and then query the bucket with [`from()`](https://v2.docs.influxdata.com/v2.0/reference/flux/stdlib/built-in/inputs/from/)._

### Write data to InfluxDB using line protocol

#### Download the line protocol load it through the InfluxDB UI
[Download the sample bird migration line protocol](https://raw.githubusercontent.com/influxdata/influxdb2-sample-data/master/bird-migration-data/bird-migration.line)
and write it to a bucket [using the InfluxDB UI](https://v2.docs.influxdata.com/v2.0/write-data/#user-interface).

#### Use the `influx` CLI
Run the following commands to download and write the sample bird migration data.
Replace `<bucket-name>` with the name of the bucket to write to.

```sh
curl https://raw.githubusercontent.com/influxdata/influxdb2-sample-data/master/bird-migration-data/bird-migration.line --output ./tmp-data
influx write -b <bucket-name> @./tmp-data
rm -f ./tmp-data
```
