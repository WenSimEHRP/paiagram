#context [
  #let plg = plugin("paiagram_wasm.wasm")
  #import "foreign/qetrc.typ": read-qetrc-2
  #set page(height: auto, width: auto)
  #set text(font: "Sarasa UI SC")
  #let trains = (
    G1000: (
      label_size: (10pt / 1pt, 10pt / 1pt),
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
    alpha: (label_size: (10pt / 1pt, 10pt / 1pt)),
    beta: (label_size: (10pt / 1pt, 10pt / 1pt)),
    charlie: (label_size: (100pt / 1pt, 10pt / 1pt)),
    delta: (label_size: (10pt / 1pt, 10pt / 1pt)),
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
        position_axis_scale_mode: "Uniform",
        time_axis_scale_mode: "Linear",
        position_axis_scale: 0.7,
        time_axis_scale: 4.0,
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

  #let pt((x, y)) = (x * 1pt, y * 1pt)


  #box(
    stroke: blue,
    width: (a.collision_manager.x_max - a.collision_manager.x_min) * 1pt,
    height: (a.collision_manager.y_max - a.collision_manager.y_min) * 1pt,
    {
      let place-curve = place.with(dx: a.collision_manager.x_min * -1pt, dy: a.collision_manager.y_min * -1pt)

      place-curve(
        block(
          width: 100% - (a.collision_manager.x_min * -1pt),
          height: 100% - (a.collision_manager.y_min * -1pt),
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
                columns: (1fr,) * 24,
                rows: (a.graph_intervals.map(it => it * 1pt).sum(), auto),
                ..range(23).map(it => place(top + left, place(bottom + center, dy: -5pt)[#it])),


                {
                  place(top + left, place(bottom + center, dy: -5pt)[23])
                  place(top + right, place(bottom + center, dy: -5pt)[24])
                }
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
                    it,
                  ),
                ))
              ),
            )
          },
        ),
      )

      for col in a.collision_manager.collisions {
        let (first, ..rest) = col
        let ops = (
          curve.move(pt(first)),
          ..rest.map(it => curve.line(pt(it))),
        )
        place-curve(
          curve(
            stroke: blue,
            fill: blue.transparentize(80%),
            ..ops,
            curve.close(),
          ),
        )
      }

      {
        for train in a.trains {
          for edge in train.edges {
            let (first, ..rest) = edge
            let ops = (
              curve.move(pt(first)),
              ..rest.map(it => curve.line(pt(it))),
            )
            place-curve(
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
            place-curve(
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
]
