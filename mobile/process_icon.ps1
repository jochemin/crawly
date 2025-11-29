
Add-Type -AssemblyName System.Drawing

$sourcePath = "C:\Users\jarenillas\.gemini\antigravity\brain\35d141be-28d8-4d9e-a887-c227832bdd7a\uploaded_image_1764053955846.png"
$destPath = "C:\crawly\mobile\assets\icon.png"

try {
    $image = [System.Drawing.Image]::FromFile($sourcePath)
    $bitmap = new-object System.Drawing.Bitmap $image
    $bitmap.Save($destPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $image.Dispose()
    $bitmap.Dispose()
    Write-Host "Image processed and saved to $destPath"
} catch {
    Write-Error "Failed to process image: $_"
    exit 1
}
