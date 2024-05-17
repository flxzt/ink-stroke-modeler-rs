#set page(width: 16cm, margin: 0.5em, height: auto)
#let definition(content) = box(fill: luma(92%), width: 100%, inset: 0.5em, stroke: black)[#content]

#import "@preview/lovelace:0.2.0": *
#show: setup-lovelace.with(body-inset: 0pt)

#let pr = $nu$
#let time = $t$


== Resampling
#let inv = $v$

*Algorithm* : Upsampling\
*Input* : ${inv[k] = (x[k],y[k],t[k]), 0 <= k <= n}$ and a target rate `sampling_min_output_rate`\,
    Interpolate time and position between each $k$ by linearly adding interpolated values
  \
  This is done by adding linearly interpolated values so that the output stream is
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
    $\
  *Output* : ${(x'[k'],y'[k'],t'[k']), 0 <= k; <= n'}$ the upsampled position and times. This verifies $
    forall k', t'[k'] - t'[k'-1] < Delta_"target" = 1/#text[`sampling_min_output_rate`]
  $\,
*Remark* : As this is a streaming algorithm, we only calculate this interpolation with respect to the latest stroke
position.