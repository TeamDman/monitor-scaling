# Path to features.json
$featuresFile = "features.json"

# Load features.json
if (-not (Test-Path $featuresFile)) {
    Write-Error "features.json not found. Run the script to download it first."
    exit 1
}

# Read and parse the entire JSON content
$featuresContent = Get-Content -Path $featuresFile -Raw
$features = $featuresContent | ConvertFrom-Json -Depth 100

# Extract namespace_map, feature_map, and namespaces
$namespaceMap = $features.namespace_map
$featureMap = $features.feature_map
$namespacesDict = $features.namespaces

# Initialize a hashtable to map namespaces to their required features
$namespaceToFeatures = @{}

Write-Host "Building namespace to features mapping..."

# Iterate over each namespace in namespace_map
for ($i = 0; $i -lt $namespaceMap.Count; $i++) {
    $namespace = $namespaceMap[$i]

    # Convert index to string since 'namespaces' keys are strings
    $key = "$i"

    # Check if the current namespace has entries in 'namespaces'
    if ($namespacesDict.PSObject.Properties.Name -contains $key) {
        $namespaceDetails = $namespacesDict.$key

        # Initialize a collection to hold feature indexes
        $featureIndexes = @()

        # Iterate over each subcomponent in the namespace
        foreach ($entry in $namespaceDetails) {
            if ($entry.features -and $entry.features.Count -gt 0) {
                $featureIndexes += $entry.features
            }
        }

        # Remove duplicate feature indexes
        $uniqueFeatureIndexes = $featureIndexes | Sort-Object -Unique

        # Map feature indexes to feature names
        $featureNames = @()
        foreach ($featureIndex in $uniqueFeatureIndexes) {
            if ($featureIndex -ge 0 -and $featureIndex -lt $featureMap.Count) {
                $featureName = $featureMap[$featureIndex]
                $featureNames += $featureName
            }
            else {
                Write-Warning "Feature index $featureIndex is out of bounds for feature_map."
            }
        }

        # Remove duplicate feature names
        $featureNames = $featureNames | Sort-Object -Unique

        # Add the namespace and its features to the hashtable
        $namespaceToFeatures[$namespace] = $featureNames
    }
    else {
        Write-Warning "No entries found in 'namespaces' for namespace_map index $i ($namespace)."
    }
}

Write-Host "Namespace to features mapping completed."

# Function to reconstruct the full namespace from the import
function Get-FullNamespace {
    param (
        [string]$importLine
    )

    # Remove trailing semicolon and whitespace
    $importLine = $importLine.TrimEnd(';').Trim()

    # Split the import by '::'
    $parts = $importLine -split "::"

    if ($parts.Count -lt 3) {
        Write-Warning "Import does not have enough parts to extract namespace: $importLine"
        return $null
    }

    # Reconstruct the namespace:
    # - For wildcard imports (ends with '*'), exclude the last part ('*')
    # - For specific imports, exclude the last part (e.g., 'HWND')
    if ($parts[-1] -eq '*') {
        $namespaceParts = $parts[1..($parts.Count - 2)]
    }
    else {
        $namespaceParts = $parts[1..($parts.Count - 2)]
    }

    # Join the parts with '.' and prefix with 'Windows.'
    $namespace = "Windows." + ($namespaceParts -join ".")

    return $namespace
}

# Run rg to find all 'use windows::' lines in .rs files
Write-Host "Searching for 'use windows::' imports in .rs files..."
$imports = rg "use windows::" --type rust --no-heading --no-line-number | ForEach-Object {
    ($_ -split "use ")[1] -replace ";", "" # Clean up the import lines
}

if (-not $imports) {
    Write-Error "No 'use windows::' imports found."
    exit 1
}

# Extract unique imports
$uniqueImports = $imports | Sort-Object -Unique

# Initialize required features array
$requiredFeatures = @()

Write-Host "Identifying required features based on imports..."
foreach ($import in $uniqueImports) {
    Write-Host "Processing import: $import"

    # Get the full namespace
    $namespace = Get-FullNamespace -importLine $import

    if ($null -eq $namespace) {
        continue
    }

    Write-Host "  -> Reconstructed Namespace: $namespace"

    # Check if the namespace exists in the mapping
    if ($namespaceToFeatures.ContainsKey($namespace)) {
        $featuresForNamespace = $namespaceToFeatures[$namespace]
        Write-Host "     Found features: $($featuresForNamespace -join ', ')"
        $requiredFeatures += $featuresForNamespace
    }
    else {
        Write-Warning "No features found for namespace: $namespace (import: $import)"
    }
}

# Deduplicate the required features
$requiredFeatures = $requiredFeatures | Sort-Object -Unique

# Output the required features
if ($requiredFeatures.Count -gt 0) {
    Write-Host "`nRequired windows-rs features:"
    $requiredFeatures | ForEach-Object { Write-Output $_ }

    # Save the required features to a file
    $requiredFeatures | Set-Content -Path "required_features.txt"
    Write-Host "Required features saved to required_features.txt"
}
else {
    Write-Warning "No required features identified."
}


# todo:
# use windows::Win32::Graphics::Gdi::DisplayConfigGetDeviceInfo;
# should have been
# use windows::Win32::Devices::Display::DisplayConfigGetDeviceInfo;
# the LLM hallucinated
# we should be able to find the correct namespace by looking at the json