// Import paiagram package
#import "../src/lib.typ": paiagram, qetrc
// Set page size to be auto for flexibility
#set page(width: auto, height: auto)

// Since qetrc.read uses the `measure` functionto provide label size information,
// we must wrap it in the #context block
#context {
  // read information from a qETRC pyetgr timetable file
  let data = qetrc.read(json("sample.pyetgr"))
  // the return type of qetrc.read should be a dictionary
  // with keys: "stations", "trains", "intervals"
  assert(type(data) == dictionary, message: "The return type of qetrc.read should be a dictionary")
  // render the timetable diagram
  paiagram(
    // here we use the ..dictionary notation to spread the dictionary
    ..data,
    // specify the stations to draw
    stations-to-draw: data.stations.keys(),
    // specify the start hour. The start hour could be any integer
    start-hour: -10,
    // specify the end hour. The end hour should be an integer,
    // however it cannot be smaller than the start hour
    end-hour: 31,
  )
}
