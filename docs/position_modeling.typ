#set page(width: 16cm, margin: 0.5em, height: auto)
#let definition(content) = box(fill: luma(92%), width: 100%, inset: 0.5em, stroke: black)[#content]

#let pr = $nu$
#let time = $t$

== Position modeling

#definition[The raw input is processed as follow

  raw input $->$ wobble smoother $->$ upsampled $->$ position modeling]

#definition[
The position of the pen is modeled as a weight connected by a spring to an anchor.

The anchor moves along the _resampled dewobbled inputs_, pulling the weight along with it accross a surface, with some
amount of friction. Euler integration is used to solve for the position of the pen.

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