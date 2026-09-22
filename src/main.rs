use routes::{health, auth, folders, shares, files, user};
use actix_web::web::scope as actix_scope;
use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer};
use std::env::var as env_var;
use actix_cors::Cors;

mod middleware;
mod database;
mod handlers;
mod models;
mod routes;
mod cache;
mod state;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = state::initialize().await;

    // Start the Actix web server
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Cors::default()
                .allowed_origin_fn(state::cors_allowed_origin_fn)
                .allowed_methods(vec!["GET", "POST", "PATCH", "PUT", "DELETE"])
                .supports_credentials()
                .allow_any_header()
                .max_age(420)
            )
            .service(
                actix_scope("/health")
                .service(health::api_health_check)
            )
            .service(
                actix_scope("/internal/callback")
                .wrap(from_fn(middleware::auth::api_key_check))
                // .service(files::callback_file_replace)
                .service(files::callback_file_upload)
            )
            .service(
                actix_scope("/internal/data")
                .wrap(from_fn(middleware::auth::api_key_check))
                // TODO: Use it in Phoenix Admin Panel or for internal monitoring
                .service(user::get_total_used_bytes)
                .service(user::get_all_servers)
            )
            .service(
                actix_scope("/public")
                .service(auth::public_session_validate_password)
                // .service(auth::public_session_generate_otp)
                // .service(auth::public_session_validate_otp)
                .service(auth::create_public_session)
                .service(
                    actix_scope("")
                    .wrap(from_fn(middleware::auth::public_auth_check))
                    .service(auth::get_public_session)
                    .service(auth::public_user_logout)
                )
            )
            .service(
                actix_scope("/auth")
                .service(auth::user_login)
                .service(auth::refresh_session)
                .service(
                    actix_scope("")
                    .wrap(from_fn(middleware::auth::auth_check))
                    .service(user::update_fcm_token)
                    .service(auth::user_logout)
                    .service(auth::get_session)
                )
            )
            .service(
                actix_scope("/user")
                .wrap(from_fn(middleware::auth::auth_check))
                .service(user::dropdown_search_user_by_email)
                .service(user::get_user_info_by_id)
                .service(user::update_user_info)
                .service(user::refresh_my_quota)
                .service(user::get_my_quota)
            )
            .service(
                actix_scope("/folders")
                .wrap(from_fn(middleware::auth::auth_check))
                .service(folders::list_folders_and_files_under)
                .service(folders::edit_folder_details)
                .service(folders::list_root_folders)
                .service(folders::create_folder)
                .service(folders::move_folder)
                // .service(folders::delete_folder_with_files)
            )
            .service(
                actix_scope("/files")
                .wrap(from_fn(middleware::auth::auth_check))
                .service(files::delete_any_file_version)
                .service(files::request_file_download)
                .service(files::create_wopi_session)
                .service(files::request_file_upload)
                .service(files::get_file_basic_info)
                .service(files::delete_full_file)
                .service(files::update_file_info)
                .service(files::move_file)
                // .service(files::copy_file_by_version)    // copy - copy only specific file version to a folder
            )
            .service(
                actix_scope("/share/public/folders")
                .wrap(from_fn(middleware::auth::public_auth_check))
                .service(shares::list_folders_and_files_under_public)
                .service(shares::create_public_folder)
                .service(shares::edit_public_folder)
                .service(shares::move_public_folder)
                // .service(folders::delete_folder_with_files)
            )
            // .service(
            //     actix_scope("/share/public/files")
            //     .wrap(from_fn(middleware::auth::public_auth_check))
                    // .service(files::update_file_info)
                    // .service(files::create_wopi_session)
            //      TODO: Most of the files endpoints to be copied under public access as well
            //     .service(files::file_operations)
            //     .service(files::get_file_info)
                // .service(files::copy_file_by_version)    // copy - copy only specific file version to a folder
                // .service(files::move_file)      // move - Move all, just change the folder_id and all ok
                // .service(files::delete_file_version)        // Delete file Version from GO API
                // .service(files::delete_file)        // Delete file from GO API
            // )
            .service(
                actix_scope("/share/external")
                .wrap(from_fn(middleware::auth::auth_check))
                .service(shares::create_external_share)
                .service(shares::delete_external_share)
                .service(shares::update_external_share)
                .service(shares::list_external_shares)
            )
            .service(
                actix_scope("/share/internal")
                .wrap(from_fn(middleware::auth::auth_check))
                .service(shares::list_folders_and_files_under_shared)
                .service(shares::create_internal_folder_share)
                .service(shares::delete_internal_folder_share)
                .service(shares::update_internal_folder_share)
                .service(shares::get_internal_share_out_info)
                .service(shares::list_sharing_out)
                .service(shares::list_sharing_in)
            )
    })
    .bind(("0.0.0.0", 8686))?
    .workers(env_var("API_WORKERS_COUNT").unwrap_or("4".to_string()).parse().unwrap())
    .run().await
}
