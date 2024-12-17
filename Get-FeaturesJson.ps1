if (-not (Test-Path features.json)) {
    # $url = "https://raw.githubusercontent.com/microsoft/windows-rs/master/crates/libs/windows/features.json"
    $url = "https://raw.githubusercontent.com/microsoft/windows-rs/0.58.0/crates/libs/windows/features.json"
    Write-Host "Getting features.json from $url"
    $features = Invoke-RestMethod -Uri $url
    $features | ConvertTo-Json -Depth 100 | Set-Content -Path "features.json"
} else {
    Write-Host "features.json already exists"
    $features = Get-Content -Path "features.json" -Raw | ConvertFrom-Json -Depth 100
}