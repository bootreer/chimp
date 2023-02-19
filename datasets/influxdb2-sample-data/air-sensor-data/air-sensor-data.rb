#! /usr/bin/ruby
require "optparse"
require "net/http"
require"openssl"
require "uri"
require "cgi"
require "time"

OptionParser.new do |opt|
  opt.banner = "Usage: air-sensor-data [OPTIONS]"

  opt.on("--help","Displays this help information.") do
    puts opt
    exit
  end
end.parse!

# Seed Data
seeds = [
  {id: 100, t: 71.2, h: 35.1, c: 0.5, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
  {id: 101, t: 71.8, h: 34.9, c: 0.5, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
  {id: 102, t: 72.0, h: 34.9, c: 0.5, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
  {id: 103, t: 71.3, h: 35.2, c: 0.4, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
  {id: 200, t: 73.6, h: 35.8, c: 0.5, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.05},
  {id: 201, t: 74.0, h: 35.2, c: 0.5, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
  {id: 202, t: 75.3, h: 35.7, c: 0.5, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
  {id: 203, t: 74.8, h: 35.9, c: 0.4, t_inc: -0.05..0.05, h_inc: -0.05..0.05, c_inc: -0.02..0.02},
]

def increment_data(data={})
  data[:t] += rand(data[:t_inc])
  data[:h] += rand(data[:h_inc])
  data[:c] += rand(data[:c_inc])

  # Avoid negative humidity and co
  if data[:h] < 0
    data[:h] = 0
  end
  if data[:c] < 0
    data[:c] = 0
  end

  return data
end

def line_protocol_batch(point_data=[], offset)
  batch = []
  now = (Time.now.to_i - ((60 * 60) - (10 * offset))) * 1000000000
  point_data.each do |v|
    batch << "airSensors,sensor_id=TLM0#{v[:id]} temperature=#{v[:t]},humidity=#{v[:h]},co=#{v[:c]} #{now}"
  end
  return batch.join("\n")
end

def send_batches(dataset=[])
    for i in 0..660
        dataset.map! { |seed| increment_data(seed) }
        puts line_protocol_batch(dataset, i)
    end
end

begin
  send_batches(seeds)
rescue Interrupt
  puts "\nStopping data stream..."
end
