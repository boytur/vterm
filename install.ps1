# Installs vterm without triggering security or quarantine warnings.
#
# Usage:
#   irm https://raw.githubusercontent.com/boytur/vterm/master/install.ps1 | iex
$ErrorActionPreference = "Stop"

$AppName = "vterm"
$Repo = "boytur/vterm"
$ReleaseUrl = "https://github.com/$Repo/releases/latest/download/$AppName-windows.zip"

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\$AppName"
$TargetExe = Join-Path $InstallDir "$AppName.exe"
$BackupExe = Join-Path $InstallDir "$AppName.exe.old"

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("vterm-install-" + [System.Guid]::NewGuid().ToString())
$ZipPath = Join-Path $TmpDir "$AppName.zip"
$ExtractDir = Join-Path $TmpDir "extracted"

try {
    New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null
    Write-Host "Downloading $AppName for Windows..."
    Invoke-WebRequest -Uri $ReleaseUrl -OutFile $ZipPath -UseBasicParsing

    Write-Host "Extracting $AppName..."
    Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force

    $SourceExe = Get-ChildItem -Path $ExtractDir -Recurse -Filter "$AppName.exe" | Select-Object -First 1
    if (-not $SourceExe) {
        throw "Downloaded archive does not contain $AppName.exe"
    }

    Write-Host "Installing to $InstallDir..."
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

    if (Test-Path $TargetExe) {
        try {
            Move-Item -Path $TargetExe -Destination $BackupExe -Force -ErrorAction Stop
        } catch {
            throw "Could not replace $TargetExe. $AppName may be running. Please close any running instances and try again."
        }
    }

    Copy-Item -Path $SourceExe.FullName -Destination $TargetExe -Force
    if (Test-Path $BackupExe) {
        Remove-Item -Path $BackupExe -Force -ErrorAction SilentlyContinue
    }

    # Unblock the executable to clear any SmartScreen Mark of the Web
    Unblock-File -Path $TargetExe -ErrorAction SilentlyContinue

    # Add to User PATH if not already present
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($UserPath -split ";") -notcontains $InstallDir) {
        $NewUserPath = if ($UserPath) { $UserPath.TrimEnd(";") + ";" + $InstallDir } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
        $env:Path = "$env:Path;$InstallDir"
    }

    # Create Start Menu shortcut
    try {
        $WScript = New-Object -ComObject WScript.Shell
        $StartMenuPrograms = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs"
        if (Test-Path $StartMenuPrograms) {
            $Shortcut = $WScript.CreateShortcut((Join-Path $StartMenuPrograms "$AppName.lnk"))
            $Shortcut.TargetPath = $TargetExe
            $Shortcut.IconLocation = "$TargetExe,0"
            $Shortcut.WorkingDirectory = [Environment]::GetFolderPath("UserProfile")
            $Shortcut.Description = "vterm terminal emulator"
            $Shortcut.Save()
        }
    } catch {
        Write-Warning "Could not create Start Menu shortcut: $_"
    }

    Write-Host ""
    Write-Host "Done. Installed $AppName to: $TargetExe" -ForegroundColor Green
    Write-Host "Start Menu shortcut created: $AppName"
    Write-Host "To launch from terminal, open a new terminal window and type: $AppName"
} finally {
    if (Test-Path $TmpDir) {
        Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
