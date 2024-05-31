#set page(width: 16cm, margin: 0.5em, height: auto)
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
  - starts with a #raw("Down") event
  - contains #raw("Move") for $k >=1$
  - ends either with a #raw("Move") or a #raw("Up") . If it is a #raw("Up") we say the input stream is _complete_
]

#definition[
  We addition define
  - $v_x, v_y, a_x, a_y$ as the velocity and acceleration.
  With the vector shorthand $v = (v_x,v_y) in RR^2$ and $a = (a_x, a_y) in RR^2$
]