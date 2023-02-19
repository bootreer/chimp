# CoinDesk Bitcoin Price Data

_[Powered by CoinDesk](https://www.coindesk.com/price/bitcoin)_

This directory contains the following files generated from the CoinDesk Bitcoin Pricing Data (https://www.coindesk.com/price/bitcoin).

### bitcoin-currentprice-annotated.csv
Latest Bitcoin pricing data from the CoinDesk API in annotated CSV format. Use the `csv.from` function to import the data into InfluxDB.

### bitcoin-historical-annotated.csv
Last 30 days of Bitcoin pricing data from the CoinDesk API in annotated CSV format. Use the `csv.from` function to import the data into InfluxDB.

### currentprice.json
Latest Bitcoin pricing data from the CoinDesk API in JSON format.

### currentprice.lp
Latest Bitcoin pricing data from the CoinDesk API in line protocol format. Use the `csv.from` function to import the data into InfluxDB.

### postprocess.js
A deno script for converting the currentprice.json file into line protocol for further processing.
