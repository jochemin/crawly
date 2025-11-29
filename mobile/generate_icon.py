from PIL import Image

img = Image.new('RGB', (1024, 1024), color = 'red')
img.save('simple_icon.png')
print("Generated simple_icon.png")
