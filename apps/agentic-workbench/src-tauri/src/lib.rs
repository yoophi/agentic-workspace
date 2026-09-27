pub mod application;
pub mod domain;
mod inbound;
mod infrastructure;
pub mod ports;

use application::appearance_preferences_service::AppearancePreferencesService;
use inbound::tauri_commands::{
    WorktreeWatcherState, acknowledge_agent_exchange, adjust_font_size_step,
    adopt_manual_orchestration_child, apply_window_title, bind_main_coordinator_run,
    bootstrap_orchestration_workspace, cancel_agent_run, cancel_current_prompt_and_send_to_run,
    cancel_orchestration_task, clear_goal, collect_orchestration_reports, create_git_worktree,
    create_goal, create_project, create_saved_prompt, declare_network_delivery,
    delegate_orchestration_goal, delete_git_worktree, delete_project, delete_saved_prompt,
    dispatch_orchestration_prompt, ensure_window_bench, get_agent_run_settings,
    get_appearance_preferences, get_goal, get_orchestration_workspace, get_workbench_connection,
    get_workbench_mode, get_worktree_changes, get_worktree_commit_detail,
    get_worktree_commit_file_diff, get_worktree_file_diff, get_worktree_git_graph,
    get_worktree_workspace_layout, handoff_orchestration_coordinator, list_agent_exchanges,
    list_agent_tool_command_candidates, list_agents, list_git_branches, list_git_remotes,
    list_git_worktrees, list_orchestration_tasks, list_projects, list_provider_sessions,
    list_recoverable_orchestration_workspaces, list_saved_prompts, list_worktree_changes,
    list_worktree_files, list_worktree_git_history, open_external_url, open_settings_window,
    open_worktree_window, read_worktree_text_file, reassign_orchestration_task,
    record_goal_progress, recover_orchestration_workspace, replay_orchestration_runtime_events,
    respond_agent_permission, respond_orchestration_input, retry_orchestration_task,
    save_agent_run_settings, save_worktree_workspace_layout, send_agent_exchange,
    send_orchestration_child_command, send_prompt_to_run, set_font_size_step,
    set_orchestration_presentation, set_run_permission_mode, start_agent_run,
    start_worktree_watcher, steer_prompt_to_run, stop_worktree_watcher, sync_agent_workspace,
    update_goal, update_project, update_saved_prompt, withdraw_network_delivery,
};
use infrastructure::{
    json_appearance_preferences_repository::JsonAppearancePreferencesRepository,
    mcp::McpServerState, workbench_http,
};
use std::sync::Arc;
use tauri::{
    Manager, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use workbench_core::application::workbench_runtime::{RuntimeAdapters, WorkbenchRuntime};

const ABOUT_MENU_ID: &str = "about-agentic-workbench";
const PREFERENCES_MENU_ID: &str = "preferences-agentic-workbench";
const PREFERENCES_ACCELERATOR: &str = "CmdOrCtrl+,";
const APP_DISPLAY_NAME: &str = "Agentic Workbench";
const APP_VERSION: &str = env!("AGENTIC_WORKBENCH_PACKAGE_VERSION");
const BUILD_COMMIT_HASH: &str = env!("AGENTIC_WORKBENCH_GIT_COMMIT_HASH");
const BUILD_COMMIT_TAG: &str = env!("AGENTIC_WORKBENCH_GIT_COMMIT_TAG");
const BUILD_COMMIT_FALLBACK: &str = "unknown";

/// 앱 command 목록. debug 빌드만 스모크 probe 보고 command를 더한다(042 R12·설계 리뷰 D3 — `generate_handler!`는
/// 항목별 `cfg`를 받지 않는다). release에는 probe command가 없다.
#[cfg(debug_assertions)]
macro_rules! app_invoke_handler {
    () => {
        tauri::generate_handler![
            list_projects,
            create_project,
            update_project,
            delete_project,
            list_saved_prompts,
            create_saved_prompt,
            update_saved_prompt,
            delete_saved_prompt,
            get_goal,
            create_goal,
            update_goal,
            clear_goal,
            record_goal_progress,
            get_agent_run_settings,
            save_agent_run_settings,
            get_appearance_preferences,
            set_font_size_step,
            adjust_font_size_step,
            get_worktree_workspace_layout,
            save_worktree_workspace_layout,
            list_git_remotes,
            list_git_branches,
            list_git_worktrees,
            list_worktree_changes,
            create_git_worktree,
            delete_git_worktree,
            get_worktree_changes,
            get_worktree_file_diff,
            list_worktree_files,
            read_worktree_text_file,
            start_worktree_watcher,
            stop_worktree_watcher,
            list_worktree_git_history,
            get_worktree_git_graph,
            get_worktree_commit_detail,
            get_worktree_commit_file_diff,
            list_agents,
            list_agent_tool_command_candidates,
            list_provider_sessions,
            open_external_url,
            open_worktree_window,
            open_settings_window,
            start_agent_run,
            cancel_agent_run,
            send_prompt_to_run,
            steer_prompt_to_run,
            cancel_current_prompt_and_send_to_run,
            set_run_permission_mode,
            respond_agent_permission,
            sync_agent_workspace,
            send_agent_exchange,
            acknowledge_agent_exchange,
            list_agent_exchanges,
            bootstrap_orchestration_workspace,
            list_recoverable_orchestration_workspaces,
            get_orchestration_workspace,
            bind_main_coordinator_run,
            delegate_orchestration_goal,
            adopt_manual_orchestration_child,
            list_orchestration_tasks,
            collect_orchestration_reports,
            set_orchestration_presentation,
            replay_orchestration_runtime_events,
            respond_orchestration_input,
            send_orchestration_child_command,
            cancel_orchestration_task,
            retry_orchestration_task,
            reassign_orchestration_task,
            handoff_orchestration_coordinator,
            dispatch_orchestration_prompt,
            recover_orchestration_workspace,
            get_workbench_connection,
            get_workbench_mode,
            apply_window_title,
            ensure_window_bench,
            declare_network_delivery,
            withdraw_network_delivery,
            infrastructure::http_probe::report_http_probe,
            infrastructure::http_probe::report_app_probe
        ]
    };
}

#[cfg(not(debug_assertions))]
macro_rules! app_invoke_handler {
    () => {
        tauri::generate_handler![
            list_projects,
            create_project,
            update_project,
            delete_project,
            list_saved_prompts,
            create_saved_prompt,
            update_saved_prompt,
            delete_saved_prompt,
            get_goal,
            create_goal,
            update_goal,
            clear_goal,
            record_goal_progress,
            get_agent_run_settings,
            save_agent_run_settings,
            get_appearance_preferences,
            set_font_size_step,
            adjust_font_size_step,
            get_worktree_workspace_layout,
            save_worktree_workspace_layout,
            list_git_remotes,
            list_git_branches,
            list_git_worktrees,
            list_worktree_changes,
            create_git_worktree,
            delete_git_worktree,
            get_worktree_changes,
            get_worktree_file_diff,
            list_worktree_files,
            read_worktree_text_file,
            start_worktree_watcher,
            stop_worktree_watcher,
            list_worktree_git_history,
            get_worktree_git_graph,
            get_worktree_commit_detail,
            get_worktree_commit_file_diff,
            list_agents,
            list_agent_tool_command_candidates,
            list_provider_sessions,
            open_external_url,
            open_worktree_window,
            open_settings_window,
            start_agent_run,
            cancel_agent_run,
            send_prompt_to_run,
            steer_prompt_to_run,
            cancel_current_prompt_and_send_to_run,
            set_run_permission_mode,
            respond_agent_permission,
            sync_agent_workspace,
            send_agent_exchange,
            acknowledge_agent_exchange,
            list_agent_exchanges,
            bootstrap_orchestration_workspace,
            list_recoverable_orchestration_workspaces,
            get_orchestration_workspace,
            bind_main_coordinator_run,
            delegate_orchestration_goal,
            adopt_manual_orchestration_child,
            list_orchestration_tasks,
            collect_orchestration_reports,
            set_orchestration_presentation,
            replay_orchestration_runtime_events,
            respond_orchestration_input,
            send_orchestration_child_command,
            cancel_orchestration_task,
            retry_orchestration_task,
            reassign_orchestration_task,
            handoff_orchestration_coordinator,
            dispatch_orchestration_prompt,
            recover_orchestration_workspace,
            get_workbench_connection,
            get_workbench_mode,
            apply_window_title,
            ensure_window_bench,
            declare_network_delivery,
            withdraw_network_delivery
        ]
    };
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(build_native_menu)
        .on_menu_event(|app, event| {
            if event.id() == ABOUT_MENU_ID {
                show_about_dialog(app);
            } else if event.id() == PREFERENCES_MENU_ID {
                if let Err(error) = infrastructure::window_manager::open_settings_window(app) {
                    show_error_dialog(app, "Could not open Settings", &error);
                }
            } else if let Ok(true) =
                infrastructure::native_window_menu::focus_window_from_menu_event(
                    app,
                    event.id().as_ref(),
                )
            {
                // Window focus menu events are handled by the native menu adapter.
            }
        })
        .setup(|_app| {
            let app_data_dir = _app
                .path()
                .app_data_dir()
                .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
            let appearance_repository =
                JsonAppearancePreferencesRepository::from_app(_app.handle())?;
            let appearance_service =
                AppearancePreferencesService::bootstrap(appearance_repository)?;
            _app.manage(appearance_service);

            // 044 T028: 기본은 외부 서버 모드 — 앱 안에 런타임을 두지 않고, 첫 창의 연결 요청 때 서버를 찾거나 띄운다
            // (안내 파일 · 소유 잠금 · 임대). `AW_WORKBENCH_MODE=embedded`만 아래 043 경로를 쓴다.
            let mode = infrastructure::workbench_mode::WorkbenchMode::from_env();
            _app.manage(mode);
            eprintln!("[workbench] mode: {}", mode.as_str());
            if mode == infrastructure::workbench_mode::WorkbenchMode::External {
                let executable = infrastructure::server_client::discover_server_executable();
                _app.manage(infrastructure::server_client::ExternalServer::new(
                    app_data_dir,
                    executable,
                ));
                finish_setup(_app);
                return Ok(());
            }

            // embedded(043 경로): 같은 데이터 디렉터리의 소유 잠금을 먼저 잡는다 — 외부 서버가 쥐고 있으면 부팅하지 않는다.
            let ownership =
                infrastructure::workbench_mode::EmbeddedOwnership::claim(&app_data_dir)?;
            // 037: 서버 런타임(Workbench)을 먼저 조립한다. 프로젝트 저장소는 이 런타임이 유일한 쓰기 주체다.
            // 040: 데스크톱 포트(창 삽입 전달)를 주입한다. run 종료 후처리와 orchestration은 core가 소유한다(041).
            // 044 T014: 런타임 → MCP → HTTP 조립은 `workbench-host`가 한다(창 무관 MCP 주입 포함).
            let desktop_bridge = infrastructure::tauri_desktop_bridge::TauriDesktopBridge::new(
                _app.handle().clone(),
            );
            let mut adapters = RuntimeAdapters::production();
            adapters.desktop = Some(desktop_bridge);
            // 042: 같은 런타임을 루프백 HTTP/WS로 연다. 기동 실패는 기록하고 앱은 계속 동작한다(FR-016).
            // 043 T052(debug 전용): 끝점 기동 실패를 주입해 창이 호환 경로로 부팅하는지 확인한다(SC-007).
            #[cfg(debug_assertions)]
            let injected_failure = std::env::var("AW_WORKBENCH_HTTP_FAIL_START").is_ok();
            #[cfg(not(debug_assertions))]
            let injected_failure = false;
            let mut options = workbench_host::assembly::HostOptions::new(
                app_data_dir,
                adapters,
                APP_VERSION,
                tauri::async_runtime::handle().inner().clone(),
            );
            options.owner = Some(ownership.identity().clone());
            if injected_failure {
                options.http = workbench_host::assembly::HttpStart::Fail(
                    "injected start failure (AW_WORKBENCH_HTTP_FAIL_START)".to_owned(),
                );
            }
            let host = workbench_host::assembly::assemble(options)
                .map_err(|error| format!("{error:#}"))?;
            let workbench_runtime: Arc<WorkbenchRuntime> = host.runtime.clone();
            if let Some(state) = &host.http
                && let Err(error) =
                    ownership.publish(host.runtime.epoch(), state.base_url(), APP_VERSION)
            {
                eprintln!("[workbench] could not write the embedded descriptor: {error}");
            }
            _app.manage(workbench_runtime);
            _app.manage(ownership);

            let mcp_state = host.mcp.clone();
            let (http_state, start_error) = match (host.http, host.http_start_error) {
                (Some(state), _) => {
                    eprintln!("[workbench-http] listening on {}", state.base_url());
                    (Some(state), None)
                }
                (None, error) => {
                    let error = error.unwrap_or_else(|| "not started".to_owned());
                    eprintln!("[workbench-http] failed to start: {error}");
                    (None, Some(error))
                }
            };
            let http = workbench_http::WorkbenchHttp {
                state: http_state,
                start_error,
                exit: workbench_http::ExitGate::default(),
            };
            #[cfg(debug_assertions)]
            infrastructure::http_probe::write_diagnostic_file(&http);
            _app.manage(http);
            _app.manage(mcp_state);
            finish_setup(_app);
            Ok(())
        })
        .on_page_load(|_webview, _payload| {
            #[cfg(debug_assertions)]
            infrastructure::http_probe::install_probe(
                _webview,
                matches!(_payload.event(), tauri::webview::PageLoadEvent::Finished),
            );
        })
        .on_window_event(|window, event| {
            // 세션 창의 위치·크기를 Worktree별로 저장한다. 이동·리사이즈는 드래그 중 연속으로
            // 들어오므로 간격을 두고 저장하고, 닫힐 때는 마지막 값을 반드시 기록한다.
            match event {
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    infrastructure::window_manager::save_session_window_bounds(window, false);
                }
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => {
                    infrastructure::window_manager::save_session_window_bounds(window, true);
                }
                _ => {}
            }
            // 세션 창 표현 상태 정리. 주체·토큰 폐기와 작업대 닫기(소유 run 취소)는 창별 처리기가 그 창의 incarnation으로
            // 한다(043 `window_lifecycle`).
            if let WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                if label.starts_with("session-") {
                    infrastructure::window_manager::forget_session_window(&label);
                    let watcher_state = window.state::<WorktreeWatcherState>();
                    let _ = watcher_state.stop_for_window(&label);
                }
                let _ = infrastructure::native_window_menu::sync_window_menu(window.app_handle());
            }
        })
        .manage(WorktreeWatcherState::new())
        .invoke_handler(app_invoke_handler!())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(on_run_event);
}

/// 두 모드 공통 설정 마무리.
fn finish_setup(app: &mut tauri::App) {
    // 043: 설정 파일로 만들어진 창도 창 주체를 등록한다(이벤트 루프 전이라 아직 호출이 없다).
    for window in app.webview_windows().values() {
        infrastructure::window_lifecycle::adopt(app.handle(), window);
    }

    #[cfg(debug_assertions)]
    {
        if infrastructure::devtools::should_open_devtools()
            && let Some(window) = app.get_webview_window("main")
        {
            window.open_devtools();
        }
    }

    let _ = infrastructure::native_window_menu::sync_window_menu(app.handle());
}

/// 044 T031: 외부 서버 모드의 종료. 작업대를 닫지 않는다(서버가 계속 소유한다) — 진행 중 창 폐기를 흘려보내고
/// 임대를 놓는다. 둘 다 짧은 상한(2초) 안에서만 기다린다. R8: 앱 Quit은 `Exit`만, 마지막 창 닫기는 `ExitRequested`로 온다.
fn exit_external(app: &tauri::AppHandle) {
    infrastructure::window_lifecycle::mark_quitting();
    let Some(server) = app.try_state::<Arc<infrastructure::server_client::ExternalServer>>() else {
        return;
    };
    let server = server.inner().clone();
    eprintln!(
        "[workbench] exit: flushing {} window retirement(s) and releasing the lease",
        server.pending_retirements()
    );
    tauri::async_runtime::block_on(
        server.release_for_exit(infrastructure::server_client::EXIT_FLUSH_LIMIT),
    );
}

/// 042 T033: 종료는 받아들인 HTTP·MCP 호출이 끝날 때까지 미룬다 — 신호만 보내고 곧바로 끝내면 분리 실행한 호출의
/// 결과 기록이 프로세스와 함께 사라진다. 두 경로가 있다:
/// - `ExitRequested`(창을 모두 닫음 등): 종료를 미루고 비동기로 drain한 뒤 같은 코드로 다시 종료한다.
/// - `Exit`(macOS 앱 메뉴 Quit·terminate는 `ExitRequested` 없이 이것만 온다 — 042 스모크에서 확인): 이벤트 루프가
///   끝나기 전에 그 자리에서 drain을 기다린다. drain은 tokio 작업자에서 돌므로 메인 스레드를 막아도 진행된다.
fn on_run_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    let external = app
        .try_state::<infrastructure::workbench_mode::WorkbenchMode>()
        .map(|mode| *mode == infrastructure::workbench_mode::WorkbenchMode::External)
        .unwrap_or(false);
    if external {
        match event {
            // 마지막 창 닫기 등: 종료를 미루지 않는다 — 폐기 흘려보내기와 임대 해제는 `Exit`에서 한 번 한다.
            tauri::RunEvent::ExitRequested { .. } => {
                infrastructure::window_lifecycle::mark_quitting()
            }
            tauri::RunEvent::Exit => exit_external(app),
            _ => {}
        }
        return;
    }
    if matches!(
        event,
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
    ) {
        infrastructure::window_lifecycle::mark_quitting();
    }
    if let tauri::RunEvent::Exit = event
        && let Some(ownership) =
            app.try_state::<infrastructure::workbench_mode::EmbeddedOwnership>()
    {
        ownership.withdraw();
    }
    match event {
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            let http = app.state::<workbench_http::WorkbenchHttp>();
            match http.exit.on_exit_requested() {
                workbench_http::ExitDecision::Exit => {}
                workbench_http::ExitDecision::KeepWaiting => api.prevent_exit(),
                workbench_http::ExitDecision::StartDrain => {
                    api.prevent_exit();
                    let app = app.clone();
                    let state = http.state.clone();
                    let mcp_calls = app.state::<McpServerState>().detached_calls();
                    eprintln!(
                        "[workbench-http] exit requested: closing new calls and draining accepted calls"
                    );
                    let runtime = app.state::<Arc<WorkbenchRuntime>>().inner().clone();
                    tauri::async_runtime::spawn(async move {
                        workbench_http::drain_for_exit(state, mcp_calls, async move {
                            runtime.close_all_benches().await;
                        })
                        .await;
                        eprintln!("[workbench-http] exit: accepted calls drained");
                        app.state::<workbench_http::WorkbenchHttp>().exit.drained();
                        app.exit(code.unwrap_or(0));
                    });
                }
            }
        }
        tauri::RunEvent::Exit => {
            let http = app.state::<workbench_http::WorkbenchHttp>();
            if http.exit.is_drained() {
                return;
            }
            eprintln!(
                "[workbench-http] exit (event loop ending): closing new calls and draining accepted calls"
            );
            let state = http.state.clone();
            let mcp_calls = app.state::<McpServerState>().detached_calls();
            let runtime = app.state::<Arc<WorkbenchRuntime>>().inner().clone();
            tauri::async_runtime::block_on(workbench_http::drain_for_exit(
                state,
                mcp_calls,
                async move {
                    runtime.close_all_benches().await;
                },
            ));
            http.exit.drained();
            eprintln!("[workbench-http] exit: accepted calls drained");
        }
        _ => {}
    }
}

fn build_native_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let about_item = MenuItem::with_id(
        app,
        ABOUT_MENU_ID,
        format!("About {APP_DISPLAY_NAME}"),
        true,
        None::<&str>,
    )?;
    let preferences_item = MenuItem::with_id(
        app,
        preferences_menu_id(),
        "Preferences...",
        true,
        Some(preferences_accelerator()),
    )?;
    let window_menu = Submenu::with_id_and_items(
        app,
        "Window",
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(
        app,
        "Help",
        "Help",
        true,
        &[
            #[cfg(not(target_os = "macos"))]
            &about_item,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                APP_DISPLAY_NAME,
                true,
                &[
                    &about_item,
                    &PredefinedMenuItem::separator(app)?,
                    &preferences_item,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            #[cfg(not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            )))]
            &Submenu::with_items(
                app,
                "File",
                true,
                &[
                    &PredefinedMenuItem::close_window(app, None)?,
                    #[cfg(not(target_os = "macos"))]
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?,
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app, None)?],
            )?,
            &window_menu,
            &help_menu,
        ],
    )
}

fn show_about_dialog<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    app.dialog()
        .message(format!(
            "{APP_DISPLAY_NAME}\n\nVersion: {APP_VERSION}\nCommit: {}\nTag: {}",
            display_commit_hash(),
            display_commit_tag()
        ))
        .title(format!("About {APP_DISPLAY_NAME}"))
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}

fn show_error_dialog<R: tauri::Runtime>(app: &tauri::AppHandle<R>, title: &str, message: &str) {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}

fn display_commit_hash() -> &'static str {
    if BUILD_COMMIT_HASH.trim().is_empty() {
        BUILD_COMMIT_FALLBACK
    } else {
        BUILD_COMMIT_HASH
    }
}

fn display_commit_tag() -> &'static str {
    if BUILD_COMMIT_TAG.trim().is_empty() {
        BUILD_COMMIT_FALLBACK
    } else {
        BUILD_COMMIT_TAG
    }
}

fn preferences_menu_id() -> &'static str {
    PREFERENCES_MENU_ID
}

fn preferences_accelerator() -> &'static str {
    PREFERENCES_ACCELERATOR
}

#[cfg(test)]
mod tests {
    use super::{preferences_accelerator, preferences_menu_id};

    #[test]
    fn preferences_menu_uses_stable_id() {
        assert_eq!(preferences_menu_id(), "preferences-agentic-workbench");
    }

    #[test]
    fn preferences_menu_uses_standard_macos_accelerator() {
        assert_eq!(preferences_accelerator(), "CmdOrCtrl+,");
    }
}
