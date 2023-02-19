# NOAA NDBC Data

This directory contains the following files generated from the NOAA National Data Bouy Center (https://www.ndbc.noaa.gov/).

### active-stations.xml
This is metadata about the active stations used to enrich the observation data below. Pulled from https://www.ndbc.noaa.gov/activestations.xml.

### latest-observations-annotated.csv
This is the latest observation data in Annotated CSV format meant to be imported into InfluxDB via the `csv.from` function.

### latest-observations.csv
This is the latest observation data meant to be imported into PostgreSQL or MySQL.

### latest-observations.lp
This is the latest observation data in Line Protocol format meant to be imported into InfluxDB via the web UI or CLI `influx write` command.

### latest-observations.txt
This is the latest observations data file which when combined with the active stations XML file above forms the basis for the csv and lp files. Pulled from https://www.ndbc.noaa.gov/data/latest_obs/latest_obs.txt.

### postprocess.js
A deno script for converting the latest-observations.txt file into csv for further processing.