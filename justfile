# build docs
doc:
    pandoc docs/notations.typ -o docs/notations.html --mathml
    pandoc docs/position_modeling.typ -o docs/position_modeling.html --mathml
    pandoc docs/resampling.typ -o docs/resampling.html --mathml
    pandoc docs/stroke_end.typ -o docs/stroke_end.html --mathml
    pandoc docs/stylus_state_modeler.typ -o docs/stylus_state_modeler.html --mathml
    pandoc docs/wobble.typ -o docs/wobble.html --mathml
    cargo doc --open

remove_html:
    rm docs/notations.html
    rm docs/position_modeling.html
    rm docs/resampling.html
    rm docs/stroke_end.html
    rm docs/stylus_state_modeler.html
    rm docs/wobble.html
