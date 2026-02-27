param(
    [Parameter(Mandatory = $true)]
    [string]$GjfJar,
    [Parameter(Mandatory = $true)]
    [string[]]$Files
)

cargo run -p gjf-reference -- --gjf-jar $GjfJar $Files
