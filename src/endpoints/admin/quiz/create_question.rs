use crate::middleware::auth::auth_middleware;
use crate::models::{QuestDocument, QuestTaskDocument, QuizInsertDocument, QuizQuestionDocument};
use crate::utils::get_next_question_id;
use crate::utils::verify_quest_auth;
use crate::{models::AppState, utils::get_error};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_auto_routes::route;
use mongodb::bson::doc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub_struct!(Deserialize; CreateQuizQuestion {
    quiz_id: i64,
    question: String,
    options:Vec<String>,
    correct_answers: Vec<i64>,
});

#[route(post, "/admin/tasks/quiz/question/create", auth_middleware)]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Extension(sub): Extension<String>,
    body: Json<CreateQuizQuestion>,
) -> impl IntoResponse {
    let quiz_collection = state.db.collection::<QuizInsertDocument>("quizzes");
    let quiz_questions_collection = state
        .db
        .collection::<QuizQuestionDocument>("quiz_questions");
    let quests_collection = state.db.collection::<QuestDocument>("quests");
    let tasks_collection = state.db.collection::<QuestTaskDocument>("tasks");

    let pipeline = doc! {
        "quiz_name": &body.quiz_id,
    };
    let res = &tasks_collection.find_one(pipeline, None).await.unwrap();
    if res.is_none() {
        return get_error("quiz does not exist".to_string());
    }

    // get the quest id
    let quest_id = res.as_ref().unwrap().id as i64;

    let res = verify_quest_auth(sub, &quests_collection, &quest_id).await;
    if !res {
        return get_error("Error creating question".to_string());
    };

    // filter to get existing quiz
    let filter = doc! {
        "id": &body.quiz_id,
    };

    let existing_quiz = &quiz_collection
        .find_one(filter.clone(), None)
        .await
        .unwrap();
    if existing_quiz.is_none() {
        return get_error("quiz does not exist".to_string());
    }

    let mut state_last_id = state.last_question_id.lock().await;

    let next_quiz_question_id =
        get_next_question_id(&quiz_questions_collection, state_last_id.clone()).await;

    *state_last_id = next_quiz_question_id;

    let new_quiz_document = QuizQuestionDocument {
        quiz_id: body.quiz_id.clone(),
        question: body.question.clone(),
        options: body.options.clone(),
        correct_answers: body.correct_answers.clone(),
        id: next_quiz_question_id,
        kind: "text_choice".to_string(),
        layout: "default".to_string(),
    };

    return match quiz_questions_collection
        .insert_one(new_quiz_document, None)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({"message": "Question created successfully"})).into_response(),
        )
            .into_response(),
        Err(_e) => return get_error("Error creating question".to_string()),
    };
}
