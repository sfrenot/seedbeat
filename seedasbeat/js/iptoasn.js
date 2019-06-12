const iptoasn = require("iptoasn")("cache/");

seeds = process.argv.slice(2)

function start() {
  seeds.forEach(function(ip) {
    res = iptoasn.lookup(ip)
    if (res) {
      elems = res.name.split(',')
      console.log(`${res.asn};${elems.slice(0, -1).join(',').trim()};${elems.slice(-1)[0].trim()}`);
    }
  })
}

iptoasn.lastUpdated(function(t) {
  if (t > 31) {
    iptoasn.update(null, () => {
      iptoasn.load(start);
    });
  } else {
    iptoasn.load(start);
  }
});
