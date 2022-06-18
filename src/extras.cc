
#include <vector>
#include <variant>
#include <string>
#include <iostream>
#include "absl/status/status.h"
#include "absl/status/statusor.h"
#include "ink_stroke_modeler/types.h"
#include "ink_stroke_modeler/params.h"
#include "ink_stroke_modeler/stroke_modeler.h"

#include "rust/cxx.h"
#include "extras.h"

// Private function definitions
static ink::stroke_model::PredictionParams convert_bd_prediction_params(BdPredictionParams bd_prediction_params);

static ink::stroke_model::StrokeModelParams convert_bd_stroke_modeler_params(
    BdStrokeModelParams bd_params);

// Function declarations

static ink::stroke_model::PredictionParams convert_bd_prediction_params(BdPredictionParams bd_prediction_params)
{
    if (std::holds_alternative<BdStrokeEndPredictorParams>(bd_prediction_params))
    {
        // BdStrokeEndPredictorParams bd_stroke_end_predictor_params = std::get<BdStrokeEndPredictorParams>(bd_prediction_params);
        return ink::stroke_model::StrokeEndPredictorParams{};
    }
    else
    {
        BdKalmanPredictorParams bd_kalman_predictor_params = std::get<BdKalmanPredictorParams>(bd_prediction_params);
        return ink::stroke_model::KalmanPredictorParams{
            .process_noise = bd_kalman_predictor_params.process_noise,
            .measurement_noise = bd_kalman_predictor_params.measurement_noise,
            .min_stable_iteration = bd_kalman_predictor_params.min_stable_iteration,
            .max_time_samples = bd_kalman_predictor_params.max_time_samples,
            .min_catchup_velocity = bd_kalman_predictor_params.min_catchup_velocity,
            .acceleration_weight = bd_kalman_predictor_params.acceleration_weight,
            .jerk_weight = bd_kalman_predictor_params.jerk_weight,
            .prediction_interval = ink::stroke_model::Duration(bd_kalman_predictor_params.prediction_interval),
            .confidence_params{
                .desired_number_of_samples = bd_kalman_predictor_params.confidence_params.desired_number_of_samples,
                .max_estimation_distance = bd_kalman_predictor_params.confidence_params.max_estimation_distance,
                .min_travel_speed = bd_kalman_predictor_params.confidence_params.min_travel_speed,
                .max_travel_speed = bd_kalman_predictor_params.confidence_params.max_travel_speed,
                .max_linear_deviation = bd_kalman_predictor_params.confidence_params.max_linear_deviation,
                .baseline_linearity_confidence = bd_kalman_predictor_params.confidence_params.baseline_linearity_confidence,
            }};
    }
}

static ink::stroke_model::StrokeModelParams convert_bd_stroke_modeler_params(
    BdStrokeModelParams bd_params)
{

    return ink::stroke_model::StrokeModelParams{
        .wobble_smoother_params{
            .timeout = ink::stroke_model::Duration(bd_params.wobble_smoother_params.timeout),
            .speed_floor = bd_params.wobble_smoother_params.speed_floor,
            .speed_ceiling = bd_params.wobble_smoother_params.speed_ceiling},
        .position_modeler_params{
            .spring_mass_constant = bd_params.position_modeler_params.spring_mass_constant,
            .drag_constant = bd_params.position_modeler_params.drag_constant},
        .sampling_params{.min_output_rate = bd_params.sampling_params.min_output_rate,
                         .end_of_stroke_stopping_distance = bd_params.sampling_params.end_of_stroke_stopping_distance,
                         .end_of_stroke_max_iterations = bd_params.sampling_params.end_of_stroke_max_iterations,
                         .max_outputs_per_call = bd_params.sampling_params.max_outputs_per_call},
        .stylus_state_modeler_params{.max_input_samples = bd_params.stylus_state_modeler_params.max_input_samples},
        .prediction_params = convert_bd_prediction_params(bd_params.prediction_params),
        .experimental_params{}};
}

BdStrokeModelParams bd_stroke_model_params_new_w_stroke_end_predictor(
    BdWobbleSmootherParams wobble_smoother_params,
    BdPositionModelerParams position_modeler_params,
    BdSamplingParams sampling_params,
    BdStylusStateModelerParams stylus_state_modeler_params)
{
    return BdStrokeModelParams{
        .wobble_smoother_params = wobble_smoother_params,
        .position_modeler_params = position_modeler_params,
        .sampling_params = sampling_params,
        .stylus_state_modeler_params = stylus_state_modeler_params,
        .prediction_params = BdStrokeEndPredictorParams{},
    };
}

BdStrokeModelParams bd_stroke_model_params_new_w_kalman_predictor(
    BdWobbleSmootherParams wobble_smoother_params,
    BdPositionModelerParams position_modeler_params,
    BdSamplingParams sampling_params,
    BdStylusStateModelerParams stylus_state_modeler_params,
    BdKalmanPredictorParams kalman_predictor_params)
{
    return BdStrokeModelParams{
        .wobble_smoother_params = wobble_smoother_params,
        .position_modeler_params = position_modeler_params,
        .sampling_params = sampling_params,
        .stylus_state_modeler_params = stylus_state_modeler_params,
        .prediction_params = kalman_predictor_params,
    };
}

ink::stroke_model::StrokeModeler stroke_modeler_new(
    BdStrokeModelParams bd_params)
{

    ink::stroke_model::StrokeModelParams params = convert_bd_stroke_modeler_params(
        bd_params);

    ink::stroke_model::StrokeModeler stroke_modeler;

    absl::Status status = stroke_modeler.Reset(params);

    if (!status.ok())
    {
        std::cout << "reset stroke modeler failed, status: "
                  << status.ToString()
                  << "\n";
    }

    return stroke_modeler;
}

void stroke_modeler_reset(
    ink::stroke_model::StrokeModeler &stroke_modeler)
{
    absl::Status status = stroke_modeler.Reset();

    if (!status.ok())
    {
        std::cout << "reset stroke modeler failed, status: "
                  << status.ToString()
                  << "\n";
    }
}

void stroke_modeler_reset_w_params(
    ink::stroke_model::StrokeModeler &stroke_modeler,
    BdStrokeModelParams bd_params)
{
    ink::stroke_model::StrokeModelParams params = convert_bd_stroke_modeler_params(
        bd_params);

    absl::Status status = stroke_modeler.Reset(params);

    if (!status.ok())
    {
        std::cout << "reset stroke modeler failed, status: "
                  << status.ToString()
                  << "\n";
    }
}

std::vector<ink::stroke_model::Result> stroke_modeler_update(ink::stroke_model::StrokeModeler &stroke_modeler, ink::stroke_model::Input input)
{
    absl::StatusOr status = stroke_modeler.Update(input);

    if (!status.ok())
    {
        std::cout << "update stroke modeler failed, status: "
                  << status.status().ToString()
                  << "\n";

        return std::vector<ink::stroke_model::Result>();
    }

    return status.value();
}

std::vector<ink::stroke_model::Result> stroke_modeler_predict(const ink::stroke_model::StrokeModeler &stroke_modeler)
{
    absl::StatusOr status = stroke_modeler.Predict();

    if (!status.ok())
    {
        std::cout << "predict with stroke modeler failed, status: "
                  << status.status().ToString()
                  << "\n";

        return std::vector<ink::stroke_model::Result>();
    }

    return status.value();
}

ink::stroke_model::Input input_new(ink::stroke_model::Input::EventType event_type, ink::stroke_model::Vec2 pos, double time, float pressure, float tilt, float orientation)
{
    return ink::stroke_model::Input{
        event_type,
        pos,
        ink::stroke_model::Time(time),
        pressure,
        tilt,
        orientation};
}

ink::stroke_model::Input::EventType input_get_event_type(const ink::stroke_model::Input &input)
{
    return input.event_type;
}

ink::stroke_model::Vec2 input_get_position(const ink::stroke_model::Input &input)
{
    return input.position;
}

double input_get_time(const ink::stroke_model::Input &input)
{
    return input.time.Value();
}

float input_get_pressure(const ink::stroke_model::Input &input)
{
    return input.pressure;
}

float input_get_tilt(const ink::stroke_model::Input &input)
{
    return input.tilt;
}

float input_get_orientation(const ink::stroke_model::Input &input)
{
    return input.orientation;
}

// cxx does not allow unique pointers in returned CxxVectors, so we need this function to make every element a unique_ptr afterwards
std::unique_ptr<ink::stroke_model::Result> result_make_unique(ink::stroke_model::Result result)
{
    return std::make_unique<ink::stroke_model::Result>(result);
}

ink::stroke_model::Vec2 result_get_position(const ink::stroke_model::Result &result)
{
    return result.position;
}

ink::stroke_model::Vec2 result_get_velocity(const ink::stroke_model::Result &result)
{
    return result.velocity;
}

double result_get_time(const ink::stroke_model::Result &result)
{
    return result.time.Value();
}

float result_get_pressure(const ink::stroke_model::Result &result)
{
    return result.pressure;
}

float result_get_tilt(const ink::stroke_model::Result &result)
{
    return result.tilt;
}

float result_get_orientation(const ink::stroke_model::Result &result)
{
    return result.orientation;
}