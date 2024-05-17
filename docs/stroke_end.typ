#set page(width: 16cm, margin: 0.5em, height: auto)
#let definition(content) = box(fill: luma(92%), width: 100%, inset: 0.5em, stroke: black)[#content]

#import "@preview/lovelace:0.2.0": *
#show: setup-lovelace.with(body-inset: 0pt)

#let pr = $nu$
#let time = $t$

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
    ],
    ind,
    [$Delta t <- (Delta t)/2$],
    [*continue* #comment[this candidate will be discarded, try again with a smaller time step instead]], 
    ded,
    [*else*],
    ind,
    [$q_0["end +1"] = (p_c, v_c, q_c)$ #comment[#text(black)[We append the result to the end of the $q_0$ vector]]],
    ded,
    [*endif*],
    [*if* $norm(p_c - p["end"]) < d_"stop"$],
    ind,
    [*return*
      #comment[We are within tolerance of the anchor, we stop iterating]],
    ded,
    [*endif*],
    no-number,
    [*Output* : ${q_o [k] = (s_o [k], v_o [k], a_o [k]), 0 <= k <= n (<= K_"max" - 1)}$],
  ),
)