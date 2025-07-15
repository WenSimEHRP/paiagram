#let plg = plugin("paiagram_wasm.wasm")
#import "foreign/qetrc.typ": read-qetrc, match-name-color
#set page(height: auto, width: auto)
#set text(font: "Sarasa Mono SC", top-edge: "bounds", bottom-edge: "bounds")

#context [
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
  ) = read-qetrc(
    json("../jingguang.pyetgr"),
    train-stroke: train => {
      import "@preview/digestify:0.1.0": *
      let a = calc.rem(int.from-bytes(md5(bytes(train.name)).slice(0, 4)), 360)
      oklch(70%, 40%, a * 1deg)
    },
    train-label: train => {
      pad(
        .1em,
        grid(
          columns: 1,
          rows: auto,
          align: center + horizon,
          gutter: .1em,
          grid(
            gutter: .1em,
            columns: 2,
            box(height: .8em, width: 1em, image("../China_Railways.svg")),
            text(
              top-edge: "cap-height",
              bottom-edge: "baseline",
            )[#train.name],
          ),

          text(size: .5em, weight: 800, scale(x: 70%, reflow: true)[#(train.raw.sfz)---#(train.raw.zdz)]),
        ),
      )
    },
  )

  #for file in ("../jinghu.pyetgr", "../examples/sample.pyetgr", "../jingha.pyetgr") {
    let (sstations, strains, sintervals) = read-qetrc(json(file))
    {
      stations += sstations
      trains += strains
      intervals += sintervals
    }
  }

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
        start_time: 0 * 60 * 60,
        end_time: 24 * 60 * 60,
        unit_length: 1cm / 1pt,
        position_axis_scale_mode: "Logarithmic",
        time_axis_scale_mode: "Linear",
        position_axis_scale: 1.5,
        time_axis_scale: 6.0,
        label_angle: 10deg.rad(),
        line_stack_space: 2pt / 1pt,
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
  #let debug = false

  #box(
    stroke: if debug { blue },
    width: (a.collision_manager.x_max - a.collision_manager.x_min) * 1pt,
    height: (a.collision_manager.y_max - a.collision_manager.y_min) * 1pt,
    {
      let place-curve = place.with(dx: a.collision_manager.x_min * -1pt, dy: a.collision_manager.y_min * -1pt)

      place-curve(
        block(
          stroke: if debug { blue + 2pt },
          width: 24 * 6.0 * 1cm,
          height: a.graph_intervals.map(it => it * 1pt).sum(),
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
                    dx: -3pt,
                    it,
                  ),
                ))
              ),
            )
          },
        ),
      )

      place-curve({
        for train in a.trains {
          for edge in train.edges {
            let (first, ..rest) = edge.edges
            let last = rest.last()
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
                  paint: trains.at(train.name).stroke,
                  cap: "round",
                  join: "round",
                ),
                ..ops,
              ),
            )

            let (start_angle, end_angle) = edge.labels.angles
            let placed_label = trains.at(train.name).placed_label
            place(
              dx: first.at(0) * 1pt,
              dy: first.at(1) * 1pt,
              rotate(origin: top + left, start_angle * 1rad, place(bottom + left, placed_label)),
            )
            place(
              dx: last.at(0) * 1pt,
              dy: last.at(1) * 1pt,
              rotate(origin: top + left, end_angle * 1rad, place(bottom + right, placed_label)),
            )
            if debug {
              for (i, pt) in edge.edges.enumerate() {
                place(
                  center + horizon,
                  dx: pt.at(0) * 1pt,
                  dy: pt.at(1) * 1pt,
                  text(size: .7em, weight: 600)[#i],
                )
              }
            }
          }
        }
      })

      if debug {
        for col in a.collision_manager.collisions {
          let (first, ..rest) = col
          let ops = (
            curve.move(pt(first)),
            ..rest.map(it => curve.line(pt(it))),
          )
          place-curve(
            curve(
              stroke: stroke(
                paint: blue,
                join: "round",
              ),
              fill: blue.transparentize(80%),
              ..ops,
              curve.close(),
            ),
          )
        }
      }
    },
  )
]
