var geoip = require('geoip-lite');
var geo = geoip.lookup(process.argv[2]);
console.log(JSON.stringify(geo))
