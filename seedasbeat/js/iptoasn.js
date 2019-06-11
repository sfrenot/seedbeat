const iptoasn = require("iptoasn")("cache/");

console.log(process.argv[2])
process.exit()
 
function start() {
  var arr = ['50.21.180.100',
    '50.22.180.100',
    '1.38.1.1',
    2733834241,
    '8.8.8.8',
    '127.0.0.1',
    'asd'
  ];
 
  arr.forEach(function(ip) {
    console.log(ip, '-', iptoasn.lookup(ip));
  })
}

iptoasn.lastUpdated(function(t) {
  // update the database if it's older than 31 days
  // you must call .load() even if you don't update the database
  if (t > 31) {
    iptoasn.update(null, () => {
      iptoasn.load(start);
    });
  } else {
    iptoasn.load(start);
  }
});
