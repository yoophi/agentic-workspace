import{invoke}from"@tauri-apps/api/core";export const trashReviewData=(scope:"recent"|"all"|`session:${string}`)=>invoke<void>("trash_review_data",{scope});
