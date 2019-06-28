var geoip = require('geoip-lite');
var geo = geoip.lookup(process.argv[2]);
tmp = geo.ll[0]
geo.ll[0] = geo.ll[1]
geo.ll[1] = tmp
console.log(JSON.stringify(geo))
