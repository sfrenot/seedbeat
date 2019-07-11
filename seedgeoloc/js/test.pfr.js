ips = [
'118.178.129.13',
'18.220.157.44',
'3.17.9.149',
'109.70.144.173',
'138.197.73.98',
'158.69.249.172',
'84.195.164.102',
'106.15.103.188',
'94.23.58.222',
'13.115.145.229',
'47.95.22.84',
'39.100.112.3',
'144.217.73.92',
'162.221.89.109',
'52.68.49.32'
]
var geoip = require('geoip-lite');
res = []
ips.forEach( function (elem) {
  tmp = geoip.lookup(elem);
  tmp.ip = elem
  //res.push({ip: elem, ville: tmp.city});
  res.push(tmp)
})

console.log(JSON.stringify(res, null, 2))
 
