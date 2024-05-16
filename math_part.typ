#set page(width: 16cm, margin: 0.5em)
#let definition(content) = box(fill: luma(92%), width: 100%, inset: 0.5em, stroke: black)[#content]

#import "@preview/lovelace:0.2.0": *
#show: setup-lovelace.with(body-inset: 0pt)

#let pr = $nu$
#let time = $t$

== Notations

#definition[
  We denote :
  - Pressure : $pr in [0,1]$
  - time : $t >=0$
  - point : defined by position $x$ and $y$ (or $p = (x,y) in RR^2$)
  - Raw inputs are denoted by a tuple $i[k] = (pr[k], t[k], x[k],y[k])$ with $k in NN$.
]

#definition[
  An _input stream_ is a sequence of raw inputs $i = (pr, t,x,y)$
  - with time $t[k]$ $arrow.tr arrow.tr$ strictly increasing
  - starts with a #raw("kDown") event
  - contains #raw("KMove") for $k >=1$
  - ends either with a #raw("kMove") or a #raw("kUp") . If it is a #raw("kUp") we say the input stream is _complete_
]

#definition[
  We addition define
  - $v_x, v_y, a_x, a_y$ as the velocity and acceleration.
  With the vector shorthand $v = (v_x,v_y) in RR^2$ and $a = (a_x, a_y) in RR^2$
]

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
        p'[j] =                                                                                                             &
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

#pagebreak()
== Resampling
#let inv = $v$

#algorithm(caption: [Upsamling], pseudocode(
  no-number,
  [*Input* : ${inv[k] = (x[k],y[k],t[k]), 0 <= k <= n}$ and a target rate `sampling_min_output_rate`],
  [
    Interpolate time and position between each $k$ by linearly adding interpolated values
  ],
  [ This is done by adding linearly interpolated values so that the output stream is
    $
      {
      inv[0], underbrace(u_0[1]\, dots\, u_0[n_0-1], "interpolated"), inv[
      1
      ], underbrace(u_1[1]\, dots\, u_1[n_1 - 1 ], "interpolated"),inv[2], dots
      }
    $
    Each $n_i$ is the minimum integer such that
    $
            & (t[i]-t[i-1]) / n_i < Delta_"target"\
      <=> & n_i = ceil((t[i+1]-t[i])/Delta_"target")
    $
    and the linear interpolation means
    $
      u_j [k] = (1 - k / n_j) inv[j] + k / n_i inv[j+1]
    $ ],
  no-number,
  [*Output* : ${(x'[k'],y'[k'],t'[k']), 0 <= k; <= n'}$ the upsampled position and times. This verifies $
    forall k', t'[k'] - t'[k'-1] < Delta_"target" = 1/#text[`sampling_min_output_rate`]
  $],
))
*Remark* : As this is a streaming algorithm, we only calculate this interpolation with respect to the latest stroke
position.

== Position modeling

#definition[The raw input is processed as follow

  raw input $->$ wobble smoother $->$ upsampled $->$ position modeling]

#definition[
The position of the pen is modeled as a weight connected by a spring to an anchor.

The anchor moves along the _resampled dewobbled inputs_, pulling the weight along with it accross a surface, with some
amount of friction. Euler integration is used to solve for hte position of the pen.

#figure(image("position_model.svg"))

The physical model that is used to model the stroke is the following
$
  (dif^2 s) / (dif t^2) = (Phi(t) - s(t)) / k_"spring" - k_"drag" (dif s) / (dif t)
$
where
- $t$ is time
- $s(t)$ is the position of the pen
- $Phi(t)$ is the position of the anchor
- $k_"spring"$ and $k_"drag"$ are constants that sets how the spring and drag occurs

$k_"spring"$ is given by `position_modeler_spring_mass_constant` and $k_"drag"$ by `position_modeler_drag_constant`

We will thus have as input the _upsampled dewobbled inputs_ taking the role of discretized $Phi(t)$ and $s(t)$ will be
the output
]

#definition[
  Modeling a stroke.
  - Input : input stream ${(p[k],t[k]), 0 <= k <=n}$
  - Output : smoothed stream ${(p_f [k],v_f [k],a_f [k]), 0 <= k <=n}$
  We define $Phi[k] = p[k]$. An euler scheme integration scheme is used with the initial conditions being $v[0] = 0$ and $p_f [0] = p[0]$ (same
  initial conditions)

  Update rule is simply
  $
    a_f [j] = (p[j]- p_f [j-1]) / k_"spring" - k_"drag" v_f [j-1]\
    v_f [j] = v_f [j-1] + (t[j]-t[j-1])a_f [j]\
    p_f [j] = p_f [j-1] + (t[j]-t[j-1])v_f [j]
  $
  The position $s[j]$ is the main thing to export but we can also export speed and acceleration if needed. We denote
  $
    q[j] = (p_f [j],v_f [j],a_f [j],t[j])
  $
  and this will be our output with $0 <= j <= n$
]

== Stroke end

The position modeling algorithm will lag behind the raw input by some distance. This algorithm iterates the previous
dynamical system a few additional time using the raw input position as the anchor to allow a `catch up` of the stroke
(though this prediction is only given by `predict`, so is not part of the `results` and becomes obsolete on the next
input).

#algorithm(
  caption: [Stroke end],
  pseudocode(
    indentation-guide-stroke: 1pt + black,
    no-number,
    [*Input*:
    - Final anchor position $p["end"] = (x["end"],y["end"])$ #comment(text(black)[From the original input stream])
    - final tip state $q_f ["end"] = (p_f ["end"]= (x_f ["end"],y_f ["end"]),v_f ["end"], a_f ["end"])$ #comment[#text(
          fill: black,
        )[returned from the physical modeling from the last section, $dot_f$ signifies that we are looking at the filtered output]]
    - $K_"max"$ max number of iterations #comment[#text(fill: black)[`sampling_end_of_stroke_max_iterations`]]
    - $Delta_"target"$ the target time delay between stroke #comment[#text(fill: black)[1/`sampling_min_output_rate`]]
    - $d_"stop"$ stopping distance #comment[#text(fill: black)[`sampling_end_of_stroke_stopping_distance`]]
    - $k_"spring"$ and $k_"drag"$ the modeling coefficients
    ],
    // we will need to have identation to make this looks good
    [*initialize* the vector $q_o$ with $q_0 [0] = lr((underbrace(p_f ["end"], p_o [0]),
      underbrace(v_f ["end"], v_o [0]), underbrace(a_f ["end"], a_0 [0])), size: #1em)$],
    [*initialize* $Delta t = Delta_"target"$],
    [*for* $1<= k <= K_"max"$],
    ind,
    [*calculate* the next candidate
      $
        a_c &= (p ["end"] - p_o ["end"])/(k_"spring") - k_"drag" v_0 ["end"]\
        v_c &= v_o [0] + Delta t a_c\
        p_c &= p_o [0] + Delta v_c
      $
    ],
    [*if* $norm(p_c - p["end"])< d_"stop"$ #comment[#text(black)[further iterations won't be able to catch up and won't move closer to the anchor, we stop here]]],
    ind,
    [*return* $q_0$],
    ded,
    [*endif*],
    [*if* 
    $angle.l p_c - p_o ["end"], p["end"] - p_o ["end"] angle.r < norm(p_c - p_o ["end"])$
    #comment[#text(black)[we've overshot the anchor, we retry with a smaller step]]
    ], ind,
    [$Delta t <- (Delta t)/2$],ded,
    [*else*],ind,
    [$q_0["end +1"] = (p_c, v_c, q_c)$ #comment[#text(black)[We append the result to the end of the $q_0$ vector]]],
    ded,
    [*endif*],
    [*if* $norm(p_c - p["end"]) < d_"stop"$],ind,[*return*
    #comment[We are within tolerance of the anchor, we stop iterating]],ded,
    [*endif*],
    no-number,
    [*Output* : ${q_o [k] = (s_o [k], v_o [k], a_o [k]), 0 <= k <= n (<= K_"max" - 1)}$],
  ),
)

== Stylus state modeler 

Up till now we have only used the raw input stream to create a new smoothed stream of positions, leaving behind the pressure attribute. This is what's done here, to model the state of the stylus for these new position based on the pressure data of the raw input strokes.

#algorithm(
  caption: [Stylus state modeler],
  pseudocode(
    no-number,
    [*Input*],
    no-number,
    [*Output*]
  )
)