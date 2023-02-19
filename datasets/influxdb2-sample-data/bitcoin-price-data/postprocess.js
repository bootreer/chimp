import { readJSON } from 'https://deno.land/x/flat@0.0.10/src/json.ts'
import { writeTXT } from 'https://deno.land/x/flat@0.0.10/src/txt.ts'
import { Point } from 'https://cdn.skypack.dev/@influxdata/influxdb-client-browser?dts'

const bitcoinJSON = await readJSON('./bitcoin-price-data/currentprice.json')

let lpLines = []

for (const [currency, priceData] of Object.entries(bitcoinJSON.bpi)) {
    let timestamp = Date.parse(bitcoinJSON.time.updated)
    let point = new Point("coindesk")
      .timestamp(timestamp*1000000)
      .tag("crypto", bitcoinJSON.chartName.toLowerCase())
      .tag("code", priceData.code)
      .tag("symbol", priceData.symbol)
      .tag("description", priceData.description)
      .floatField("price", priceData.rate_float)

      lpLines.push(point.toLineProtocol())
}

const newFilename = `./bitcoin-price-data/currentprice.lp`
await writeTXT(newFilename, lpLines.join('\n'))