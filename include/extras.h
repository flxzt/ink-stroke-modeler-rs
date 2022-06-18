#pragma once

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

// Types
struct BdWobbleSmootherParams
{
    double timeout = -1;
    float speed_floor = -1;
    float speed_ceiling = -1;
};

struct BdPositionModelerParams
{
    float spring_mass_constant = 11.f / 32400;
    float drag_constant = 72.f;
};

struct BdSamplingParams
{
    double min_output_rate = -1;
    float end_of_stroke_stopping_distance = -1;
    int end_of_stroke_max_iterations = 20;
    int max_outputs_per_call = 100000;
};

struct BdStylusStateModelerParams
{
    int max_input_samples = 10;
};

struct BdStrokeEndPredictorParams
{
};

struct BdKalmanPredictorConfidenceParams
{
    int desired_number_of_samples = 20;
    float max_estimation_distance = -1;
    float min_travel_speed = -1;
    float max_travel_speed = -1;
    float max_linear_deviation = -1;
    float baseline_linearity_confidence = .4;
};

struct BdKalmanPredictorParams
{
    double process_noise = -1;
    double measurement_noise = -1;
    int min_stable_iteration = 4;
    int max_time_samples = 20;
    float min_catchup_velocity = -1;
    float acceleration_weight = .5;
    float jerk_weight = .1;
    double prediction_interval = -1;
    BdKalmanPredictorConfidenceParams confidence_params;
};

using BdPredictionParams =
    std::variant<BdStrokeEndPredictorParams, BdKalmanPredictorParams>;

struct BdStrokeModelParams
{
    BdWobbleSmootherParams wobble_smoother_params;
    BdPositionModelerParams position_modeler_params;
    BdSamplingParams sampling_params;
    BdStylusStateModelerParams stylus_state_modeler_params;
    BdPredictionParams prediction_params =
        BdStrokeEndPredictorParams{};
};

// Functions

BdStrokeModelParams bd_stroke_model_params_new_w_stroke_end_predictor(
    BdWobbleSmootherParams wobble_smoother_params,
    BdPositionModelerParams position_modeler_params,
    BdSamplingParams sampling_params,
    BdStylusStateModelerParams stylus_state_modeler_params);

BdStrokeModelParams bd_stroke_model_params_new_w_kalman_predictor(
    BdWobbleSmootherParams wobble_smoother_params,
    BdPositionModelerParams position_modeler_params,
    BdSamplingParams sampling_params,
    BdStylusStateModelerParams stylus_state_modeler_params,
    BdKalmanPredictorParams kalman_predictor_params);

ink::stroke_model::StrokeModeler stroke_modeler_new(
    BdStrokeModelParams bd_params);

void stroke_modeler_reset(
    ink::stroke_model::StrokeModeler &stroke_modeler);

void stroke_modeler_reset_w_params(
    ink::stroke_model::StrokeModeler &stroke_modeler,
    BdStrokeModelParams bd_params);

std::vector<ink::stroke_model::Result> stroke_modeler_update(ink::stroke_model::StrokeModeler &stroke_modeler, ink::stroke_model::Input input);

std::vector<ink::stroke_model::Result> stroke_modeler_predict(const ink::stroke_model::StrokeModeler &stroke_modeler);

ink::stroke_model::Input input_new(ink::stroke_model::Input::EventType event_type, ink::stroke_model::Vec2 pos, double time, float pressure, float tilt, float orientation);
ink::stroke_model::Input::EventType input_get_event_type(const ink::stroke_model::Input &input);
ink::stroke_model::Vec2 input_get_position(const ink::stroke_model::Input &input);
double input_get_time(const ink::stroke_model::Input &input);
float input_get_pressure(const ink::stroke_model::Input &input);
float input_get_tilt(const ink::stroke_model::Input &input);
float input_get_orientation(const ink::stroke_model::Input &input);

std::unique_ptr<ink::stroke_model::Result> result_make_unique(ink::stroke_model::Result result);
ink::stroke_model::Vec2 result_get_position(const ink::stroke_model::Result &result);
ink::stroke_model::Vec2 result_get_velocity(const ink::stroke_model::Result &result);
double result_get_time(const ink::stroke_model::Result &result);
float result_get_pressure(const ink::stroke_model::Result &result);
float result_get_tilt(const ink::stroke_model::Result &result);
float result_get_orientation(const ink::stroke_model::Result &result);