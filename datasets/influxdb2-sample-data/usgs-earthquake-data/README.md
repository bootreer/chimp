# USGS Earthquake Data

This directory contains the following files generated from the USGS Earthquake Data (https://earthquake.usgs.gov/earthquakes/feed/v1.0/geojson.php).

### all_week-annotated.csv
This is the earthquake data for the past week in Annotated CSV format meant to be imported into InfluxDB via the `csv.from` function.

### all_week.lp
This is the earthquake data in Line Protocol format meant to be imported into InfluxDB via the web UI or CLI `influx write` command.

### all_week.geojson
This is the earthquake data for the past week which forms the basis for the csv and lp files. Pulled from https://earthquake.usgs.gov/earthquakes/feed/v1.0/geojson.php.

### postprocess.js
A deno script for converting the earthquake data file into line protocol for further processing.

### schemas
The [schemas](schemas) directory contains measurement schemas for this dataset. You can create a bucket and apply the schemas with the following commands:

```
influx bucket create -n usgs_data -r 7d --schema-type explicit
influx bucket-schema create -n usgs_data --name "chemical explosion" --columns-file ./schemas/chemical-explosion-schema.ndjson --columns-format ndjson
influx bucket-schema create -n usgs_data --name "earthquake" --columns-file ./schemas/earthquake-schema.ndjson --columns-format ndjson
influx bucket-schema create -n usgs_data --name "explosion" --columns-file ./schemas/explosion-schema.ndjson --columns-format ndjson
influx bucket-schema create -n usgs_data --name "ice quake" --columns-file ./schemas/ice-quake-schema.ndjson --columns-format ndjson
influx bucket-schema create -n usgs_data --name "quarry blast" --columns-file ./schemas/quarry-blast-schema.ndjson --columns-format ndjson
```