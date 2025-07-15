#import "../src/lib.typ": paiagram
#import "../src/lib.typ": qetrc
#set text(font: "Sarasa Mono SC", top-edge: "bounds", bottom-edge: "bounds")
#set page(width: auto, height: auto)

#context {
  let (stations, trains, intervals) = qetrc.read(json("sample.pyetgr"))
  paiagram(
    stations: stations,
    trains: trains,
    intervals: intervals,
    stations-to-draw: stations.keys(),
  )
}
