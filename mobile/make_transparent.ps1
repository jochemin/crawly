
Add-Type -AssemblyName System.Drawing

$sourcePath = "c:\crawly\images\crawly.jpg"
$destPath = "C:\crawly\mobile\assets\crawly_transparent.png"

try {
    # Load the image
    $image = [System.Drawing.Image]::FromFile($sourcePath)
    $bitmap = new-object System.Drawing.Bitmap $image
    
    # Create a new bitmap with transparency support
    $transparent = new-object System.Drawing.Bitmap $bitmap.Width, $bitmap.Height
    
    # Define the background color to make transparent (white in this case)
    $bgColor = [System.Drawing.Color]::FromArgb(255, 255, 255, 255) # White
    $threshold = 200 # Threshold for considering a pixel as "white"
    
    # Process each pixel
    for ($x = 0; $x -lt $bitmap.Width; $x++) {
        for ($y = 0; $y -lt $bitmap.Height; $y++) {
            $pixel = $bitmap.GetPixel($x, $y)
            
            # Check if pixel is close to white (background)
            if ($pixel.R -gt $threshold -and $pixel.G -gt $threshold -and $pixel.B -gt $threshold) {
                # Make it transparent
                $transparent.SetPixel($x, $y, [System.Drawing.Color]::Transparent)
            }
            else {
                # Keep the original pixel
                $transparent.SetPixel($x, $y, $pixel)
            }
        }
    }
    
    # Save as PNG with transparency
    $transparent.Save($destPath, [System.Drawing.Imaging.ImageFormat]::Png)
    
    # Cleanup
    $image.Dispose()
    $bitmap.Dispose()
    $transparent.Dispose()
    
    Write-Host "Transparent image created at $destPath"
}
catch {
    Write-Error "Failed to create transparent image: $_"
    exit 1
}
