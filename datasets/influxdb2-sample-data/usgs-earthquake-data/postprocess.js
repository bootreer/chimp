import { readJSON } from 'https://deno.land/x/flat@0.0.10/src/json.ts'
import { writeTXT } from 'https://deno.land/x/flat@0.0.10/src/txt.ts'
import { Point } from 'https://cdn.skypack.dev/@influxdata/influxdb-client-browser?dts'

const earthquakeJSON = await readJSON('./usgs-earthquake-data/all_week.geojson')

let lpLines = []
earthquakeJSON.features.forEach((feature, index) => {
    let point = new Point(feature.properties.type)
      .timestamp(feature.properties.time * 1000000)
      .floatField('lon', feature.geometry.coordinates[0])
      .floatField('lat', feature.geometry.coordinates[1])
      .floatField('depth', feature.geometry.coordinates[2])
      .stringField('place', feature.properties.place)
      .stringField('url', feature.properties.url)
      .stringField('detail', feature.properties.detail)
      .stringField('alert', feature.properties.alert)
      .stringField('status', feature.properties.status)
      .stringField('ids', feature.properties.ids)
      .stringField('sources', feature.properties.sources)
      .stringField('types', feature.properties.types)
      .tag('net', feature.properties.net)
      .tag('code', feature.properties.code)
      .tag('magType', feature.properties.magType)
      .tag('title', feature.properties.title)
      .tag('id', feature.id)
    
    if(typeof feature.properties.mag === "number") {
        point = point.floatField('mag', feature.properties.mag)
    }
    if(typeof feature.properties.felt === "number") {
        point = point.intField('felt', feature.properties.felt)
    }
    if(typeof feature.properties.cdi === "number") {
        point = point.floatField('cdi', feature.properties.cdi)
    }
    if(typeof feature.properties.mmi === "number") {
        point = point.floatField('mmi', feature.properties.mmi)
    }
    if(typeof feature.properties.tsunami === "number") { 
        point = point.intField('tsunami', feature.properties.tsunami)
    }
    if(typeof feature.properties.sig === "number") {
        point = point.intField('sig', feature.properties.sig)
    }
    if(typeof feature.properties.nst === "number") { 
        point = point.intField('nst', feature.properties.nst)
    }
    if(typeof feature.properties.dmin === "number") {
        point = point.floatField('dmin', feature.properties.dmin)
    }
    if(typeof feature.properties.rms === "number") {
        point = point.floatField('rms', feature.properties.rms)
    }
    if(typeof feature.properties.gap === "number") {
        point = point.floatField('gap', feature.properties.gap)
    }
    lpLines.push(point.toLineProtocol())
})

const newFilename = `./usgs-earthquake-data/all_week.lp`
await writeTXT(newFilename, lpLines.join('\n'))
