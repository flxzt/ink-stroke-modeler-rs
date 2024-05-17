#set page(width: 16cm, margin: 0.5em, height: auto)
#let definition(content) = box(fill: luma(92%), width: 100%, inset: 0.5em, stroke: black)[#content]

#import "@preview/lovelace:0.2.0": *
#show: setup-lovelace.with(body-inset: 0pt)

#let pr = $nu$
#let time = $t$


= Wobble smoothing

To reduce high frequency noise.

#algorithm(
  caption: "Wobble smoothing",
  pseudocode(
    no-number,
    align(
      left,
    )[*Input* : ${(x[k],y[k],t[k]) in RR^2 times RR_+, 0 <= k<=n}$, $Delta T>0$ (from `wobble_smoother_timeout`), $v_"min"$ (from
    `wobble_smoother_speed_floor`) and $v_"max"$ ( from `wobble_smoother_speed_ceiling`) ],
    [
      Compute a weighted moving average of the positions $overline(p)[j] = (overline(x)[j],overline(y)[j])$
    ],
    [
      $
        forall j in [|\0,n|],
        overline(p)[
        j
        ] = cases(
          (display(sum_(k=1)^n p[k] (t[k] - t[k-1]) bb(1)_(lr([t[j] - Delta T, t[j]], size: #170%)) (t[k]))) /display(sum_(k=1)^n bb(1)_(lr([t[j] - Delta T, t[j]], size: #170%)) (t[k])) &"if the numerator" !=0,
          p[j]&"otherwise",

        )
      $
    ],
    [
      Calculate a moving average velocity $overline(v)[j]$
    ],
    [
      $
        forall j in [|0, n|],
        overline(v)[
        j
        ] = cases(0 & j = 0, (
        display(sum_(k=1)^n norm(p[k] - p[k-1]) bb(1)_(
        lr([t[j] - Delta T, t[j]], size: #170%)) (t[k]))
        )/(
        display(sum_(k=1)^n (t[k] - t[k-1]) bb(1)_(
        lr([t[j] - Delta T, t[j]], size: #170%)) (t[k]))
        )quad &"otherwise")
      $
    ],
    [
      Interpolate between the average position and the raw ones based on the average speed
    ],
    [
      $
        forall j in [|0,n|],
        p'[j] =                                                                                                               &
        min((overline(v)[j] - v_"min")/(v_"max" - v_"min") bb(1)_(\[v_"min",oo\[) (overline(v)[j]), 1) overline(p)[
        j
        ] \ + &(1 - min((overline(v)[j] - v_"min")/(v_"max" - v_"min") bb(1)_(\[v_"min",oo\[) (overline(v)[j]))) p[j]
      $
      where $p'[j] = (x'[j],y'[j])$
    ],
    [],
    no-number,
    [*Output*: ${(x'[k],y'[k]) in RR^2, 0<= k <=n}$ the filtered positions],
  ),
)
Hence for low local speeds, the smoothing is maximum (we take exactly the average position over the time $Delta T$) and
for high speed there is no smoothing. We also note that the first position is thus never filtered.
