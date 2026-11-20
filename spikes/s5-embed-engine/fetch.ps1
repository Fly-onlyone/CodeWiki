# Downloads the three catalog candidates at pinned revisions into .\models\.
# -Weights also fetches model.safetensors (95 MB, 298 MB, 1.19 GB) and checks
# its sha256 against the value the Hugging Face API reported on 2026-11-19.
param([switch]$Weights)

$ErrorActionPreference = "Stop"
$models = @(
    @{
        Dir = "granite-small"; Repo = "ibm-granite/granite-embedding-small-english-r2"
        Rev = "2ab6fa8ea2d674564defd37171ae19079b864b33"
        Sha = "dcbfaf2ee50d358763cbc0c08e5b045ecbf6aeb02dc0e86ffb910029bf9ebb5f"
        Files = @("config.json", "1_Pooling/config.json", "modules.json", "README.md", "tokenizer.json", "sentence_bert_config.json", "tokenizer_config.json", "special_tokens_map.json")
    },
    @{
        Dir = "gte-modernbert"; Repo = "Alibaba-NLP/gte-modernbert-base"
        Rev = "e7f32e3c00f91d699e8c43b53106206bcc72bb22"
        Sha = "3e85899d5728cb7de79781c0c3acfb91ccef9f875f1f7e0b3c9f3dd4b6a724ba"
        Files = @("config.json", "1_Pooling/config.json", "modules.json", "README.md", "tokenizer.json", "config_sentence_transformers.json", "tokenizer_config.json")
    },
    @{
        Dir = "qwen3-0.6b"; Repo = "Qwen/Qwen3-Embedding-0.6B"
        Rev = "97b0c614be4d77ee51c0cef4e5f07c00f9eb65b3"
        Sha = "0437e45c94563b09e13cb7a64478fc406947a93cb34a7e05870fc8dcd48e23fd"
        Files = @("config.json", "1_Pooling/config.json", "modules.json", "README.md", "tokenizer.json", "config_sentence_transformers.json", "tokenizer_config.json")
    }
)

foreach ($model in $models) {
    $dir = Join-Path $PSScriptRoot "models\$($model.Dir)"
    New-Item -ItemType Directory -Force (Join-Path $dir "1_Pooling") | Out-Null
    $files = @($model.Files)
    if ($Weights) { $files += "model.safetensors" }
    foreach ($file in $files) {
        $target = Join-Path $dir $file
        if (Test-Path $target) { continue }
        $url = "https://huggingface.co/$($model.Repo)/resolve/$($model.Rev)/$file"
        Write-Host "fetching $($model.Dir)/$file"
        # curl resumes a part file; Invoke-WebRequest does not.
        & curl.exe -sSL --fail -C - -o "$target.part" $url
        if ($LASTEXITCODE -ne 0) { throw "download failed: $url" }
        Move-Item -Force "$target.part" $target
    }
    if ($Weights) {
        $got = (Get-FileHash -Algorithm SHA256 (Join-Path $dir "model.safetensors")).Hash.ToLower()
        if ($got -ne $model.Sha) { throw "$($model.Dir): sha256 is $got, expected $($model.Sha)" }
        Write-Host "$($model.Dir): sha256 ok"
    }
}
