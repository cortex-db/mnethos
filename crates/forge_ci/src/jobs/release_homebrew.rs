use gh_workflow::*;

/// Create a homebrew release job
pub fn release_homebrew_job() -> Job {
    Job::new("homebrew_release")
        // The homebrew channel (repo cortex-db/homebrew-mnethos + HOMEBREW_ACCESS
        // secret) is not yet provisioned, so allow this job to fail without
        // failing the overall release run. It starts publishing automatically
        // once the repository and secret exist.
        .continue_on_error(true)
        .add_step(
            Step::new("Checkout Code").uses("actions", "checkout", "v6")
                .add_with(("repository", "cortex-db/homebrew-mnethos"))
                .add_with(("ref", "main"))
                .add_with(("token", "${{ secrets.HOMEBREW_ACCESS }}")),
        )
        // Make script executable and run it with token
        .add_step(
            Step::new("Update Homebrew Formula").run("GITHUB_TOKEN=\"${{ secrets.HOMEBREW_ACCESS }}\" ./update-formula.sh ${{ github.event.release.tag_name }}"),
        )
}
