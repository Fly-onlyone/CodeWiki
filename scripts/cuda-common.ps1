# What the scripts that build and ship the CUDA worker share. Dot-source it:
#   . (Join-Path $PSScriptRoot "cuda-common.ps1")

# The NVIDIA libraries the CUDA worker loads: the two it imports, and
# cublasLt, which cublas imports. They are named for CUDA 13.4.
$cudaLibraries = "curand64_10.dll", "cublas64_13.dll", "cublasLt64_13.dll"

# Names of the NVIDIA libraries a program asks Windows for when it starts,
# read from the file itself.
function Get-CudaImports([string]$Program) {
    $text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($Program))
    @([regex]::Matches($text, '(?i)\b(cu[a-z]+64_\d+|nvcuda|nvrtc[\w-]*)\.dll') |
        ForEach-Object { $_.Value.ToLower() } | Sort-Object -Unique)
}
