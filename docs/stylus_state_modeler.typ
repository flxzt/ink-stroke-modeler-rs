#set page(width: 16cm, margin: 0.5em, height: auto)
#let definition(content) = box(fill: luma(92%), width: 100%, inset: 0.5em, stroke: black)[#content]

#let comment(body) = [#text(size: 0.8em)[(#body)]]

#let pr = $nu$
#let time = $t$

== Stylus state modeler

Up till now we have only used the raw input stream to create a new smoothed stream of positions, leaving behind the
pressure attribute. This is what's done here, to model the state of the stylus for these new position based on the
pressure data of the raw input strokes.\

*Algorithm* #[Stylus state modeler]\
#[*Input* :
- input stream with pressure information ${(p[k]=(x[k],y[k]),pr[k]),0 <=k <= n}$
- query position $q = (x,y)$
- search window $n_"search"$ #comment[From `stylus_state_modeler_max_input_samples`],
]\
#[*initialize* $d = oo$, $"index"="None"$, $"interp" = "None"$]\
#[*for* $i=n-n_"search"$ to $n-1$ *do*]\
- #[*Find* $q_i$ the position that's closest to $q$ on the segment $[p[i],p[i+1]]$ and denote $r in [0,1]$ the value such
    that $
      q_i = (1-r) p[i] + r p[i+1]$]\
- #[*if* $norm(q - q_i) < d$]\
   - #[$d <- norm(q - q_i) < d\
      "index" =i\
      "interp" = r $]\
- #[*endif*]\
#[*endfor*]\
#[*calculate* $
    pr = (1-r) pr["index"] + r pr["index" +1]
  $]\
#[*Output* : interpolated pressure $pr$]
