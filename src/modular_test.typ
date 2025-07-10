#let plg = plugin("paiagram_wasm.wasm")
#import "foreign/qetrc.typ": read-qetrc-2
#set page(height: auto, width: auto)
#set text(font: "Sarasa UI SC")
#let trains = (
  G1000: (
    schedule: (
      (
        arrival: 0,
        departure: 60 * 3,
        station: "alpha",
      ),
      (
        arrival: 60 * 15,
        departure: 60 * 18,
        station: "beta",
      ),
      (
        arrival: 60 * 24,
        departure: 60 * 29,
        station: "charlie",
      ),
      (
        arrival: 60 * 35,
        departure: 60 * 45,
        station: "alpha",
      ),
      (
        arrival: 60 * 55,
        departure: 60 * 70,
        station: "beta",
      ),
      (
        arrival: 60 * 75,
        departure: 60 * 90,
        station: "alpha",
      ),
      (
        arrival: 60 * 100,
        departure: 60 * 110,
        station: "delta",
      ),
      (
        arrival: 60 * 120,
        departure: 60 * 130,
        station: "alpha",
      ),
    ),
  ),
)

#let stations = (
  alpha: (:),
  beta: (:),
  charlie: (:),
  delta: (:),
)

#let stations-to-draw = (
  "alpha",
  "beta",
  "charlie",
  "alpha",
  "delta",
  "beta",
)

#let intervals = (
  (("alpha", "beta"), (length: 1000)),
  (("beta", "charlie"), (length: 1000)),
  (("charlie", "alpha"), (length: 1000)),
  (("alpha", "delta"), (length: 1500)),
  (("delta", "charlie"), (length: 1000)),
  (("delta", "beta"), (length: 1000)),
)

#let (
  stations,
  trains,
  intervals,
) = read-qetrc-2(json("../jinghu.pyetgr"))

#let station-name-max-width = calc.max(..stations-to-draw.map(it => it.len()))

#let stations-to-draw = stations.keys()

#let a = cbor(
  plg.process(
    cbor.encode((
      stations: stations,
      trains: trains,
      intervals: intervals,
    )),
    cbor.encode((
      stations_to_draw: stations-to-draw,
      beg: 0,
      end: 24 * 60 * 60,
      unit_length: 1cm / 1pt,
      position_axis_scale_mode: "Logarithmic",
      time_axis_scale_mode: "Linear",
      position_axis_scale: 1.0,
      time_axis_scale: 7.0,
    )),
  ),
)

#let distr(s, w: auto) = {
  block(
    width: w,
    stack(
      dir: ltr,
      ..s.clusters().map(x => [#x]).intersperse(1fr),
    ),
  )
}

#box(
  width: (a.collision.x_max - a.collision.x_min) * 1pt,
  height: (a.collision.y_max - a.collision.y_min) * 1pt,
  {
    place(
      grid(
        columns: (1fr,) * 24 * 6,
        rows: a.graph_intervals.map(it => it * 1pt),
        stroke: gray,
        ..range(24 * 6).map(it => grid.vline(
          x: it,
          stroke: stroke(
            paint: gray,
            dash: "loosely-dotted",
          ),
        )),
        ..range(24 * 2).map(it => grid.vline(
          x: it * 3,
          stroke: stroke(
            paint: gray,
            dash: "densely-dotted",
          ),
        )),
        ..range(24).map(it => grid.vline(
          x: it * 6,
          stroke: stroke(
            paint: gray,
            dash: "solid",
          ),
        ))
      ),
    )

    place(
      grid(
        columns: 1fr,
        rows: a.graph_intervals.map(it => it * 1pt),
        ..stations-to-draw.map(it => place(
          top + left,
          place(
            horizon + right,
            dx: -5pt,
            distr(it, w: station-name-max-width * 1em),
          ),
        ))
      ),
    )

    place(
      grid(
        columns: (1fr,) * 24,
        rows: (a.graph_intervals.map(it => it * 1pt).sum(), auto),
        ..range(23).map(it => place(top + left, place(bottom + center, dy: -5pt)[#it])),


        {
          place(top + left, place(bottom + center, dy: -5pt)[23])
          place(top + right, place(bottom + center, dy: -5pt)[24])
        }
      ),
    )

    let pt((x, y)) = (x * 1pt, y * 1pt)

    {
      for train in a.trains {
        for edge in train.edges {
          let (first, ..rest) = edge
          let ops = (
            curve.move(pt(first)),
            ..rest.map(it => curve.line(pt(it))),
          )
          place(
            curve(
              stroke: stroke(
                paint: white,
                thickness: 2pt,
                cap: "round",
                join: "round",
              ),
              ..ops,
            ),
          )
          place(
            curve(
              stroke: stroke(
                paint: red,
                cap: "round",
                join: "round",
              ),
              ..ops,
            ),
          )
        }
      }
    }
  },
)
