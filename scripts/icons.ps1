Add-Type -AssemblyName System.Drawing
$iconFolder = Join-Path $PSScriptRoot '..\src-tauri\icons'
foreach ($iconSize in @(32,128,256)) {
  $bitmap = New-Object System.Drawing.Bitmap($iconSize,$iconSize)
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $graphics.Clear([System.Drawing.Color]::FromArgb(54,126,114))
  $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::White,($iconSize * 0.065))
  $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
  $graphics.DrawLine($pen,($iconSize*.29),($iconSize*.52),($iconSize*.44),($iconSize*.67))
  $graphics.DrawLine($pen,($iconSize*.44),($iconSize*.67),($iconSize*.73),($iconSize*.33))
  $bitmap.Save((Join-Path $iconFolder ($iconSize.ToString()+'x'+$iconSize+'.png')),[System.Drawing.Imaging.ImageFormat]::Png)
  if ($iconSize -eq 256) {
    $appIcon = [System.Drawing.Icon]::FromHandle($bitmap.GetHicon())
    $stream = [System.IO.File]::Create((Join-Path $iconFolder 'icon.ico'))
    $appIcon.Save($stream)
    $stream.Dispose()
    $appIcon.Dispose()
  }
  $graphics.Dispose(); $pen.Dispose(); $bitmap.Dispose()
}
