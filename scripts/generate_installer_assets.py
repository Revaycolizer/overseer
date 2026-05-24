#!/usr/bin/env python3
import os
import sys
import subprocess

# Ensure Pillow is installed
try:
    from PIL import Image, ImageDraw
except ImportError:
    print("[INFO] Pillow not found. Installing Pillow...")
    subprocess.check_call([sys.executable, "-m", "pip", "install", "Pillow"])
    from PIL import Image, ImageDraw

def create_gradient_vertical(size, color1, color2):
    """Creates a vertical gradient image."""
    width, height = size
    base = Image.new("RGB", (width, height), color1)
    draw = ImageDraw.Draw(base)
    
    for y in range(height):
        # Linearly interpolate between color1 and color2
        r = int(color1[0] + (color2[0] - color1[0]) * (y / height))
        g = int(color1[1] + (color2[1] - color1[1]) * (y / height))
        b = int(color1[2] + (color2[2] - color1[2]) * (y / height))
        draw.line([(0, y), (width, y)], fill=(r, g, b))
        
    return base

def main():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    assets_dir = os.path.join(repo_root, "assets")
    
    icon_path = os.path.join(assets_dir, "icon.png")
    if not os.path.exists(icon_path):
        icon_path = os.path.join(assets_dir, "overseer.png")
        
    if not os.path.exists(icon_path):
        print(f"[ERROR] Source icon not found in {assets_dir}")
        sys.exit(1)
        
    print(f"[INFO] Using source icon: {icon_path}")
    source_icon = Image.open(icon_path)
    
    # ----------------------------------------------------
    # 1. Generate installer_welcome.bmp (164 x 314 pixels)
    # ----------------------------------------------------
    welcome_size = (164, 314)
    # Premium deep dark-blue/violet gradient colors
    top_color = (15, 23, 42)     # #0F172A (Slate 900)
    bottom_color = (30, 27, 75)   # #1E1B4B (Indigo 950)
    
    welcome_img = create_gradient_vertical(welcome_size, top_color, bottom_color)
    
    # Resize and place the logo in the center of the welcome banner
    # Logo size: 80x80 pixels
    logo_size = 80
    # Keep aspect ratio
    logo = source_icon.convert("RGBA")
    logo.thumbnail((logo_size, logo_size), Image.Resampling.LANCZOS)
    
    # Calculate centering coordinates
    logo_w, logo_h = logo.size
    x_offset = (welcome_size[0] - logo_w) // 2
    y_offset = (welcome_size[1] - logo_h) // 2 - 20 # slightly above center
    
    # Paste logo onto gradient background (handling transparency mask)
    welcome_img.paste(logo, (x_offset, y_offset), logo)
    
    welcome_out_path = os.path.join(assets_dir, "installer_welcome.bmp")
    welcome_img.save(welcome_out_path, "BMP")
    print(f"[SUCCESS] Welcome banner saved to {welcome_out_path}")
    
    # ----------------------------------------------------
    # 2. Generate installer_header.bmp (150 x 57 pixels)
    # ----------------------------------------------------
    header_size = (150, 57)
    # Plain white background to merge seamlessly with NSIS header
    header_img = Image.new("RGB", header_size, (255, 255, 255))
    
    # Resize and place the logo on the right side of the header banner
    # Logo size: 40x40 pixels
    logo_size_header = 40
    logo_header = source_icon.convert("RGBA")
    logo_header.thumbnail((logo_size_header, logo_size_header), Image.Resampling.LANCZOS)
    
    logo_hw, logo_hh = logo_header.size
    # Align to right with margin of 15px
    x_offset_header = header_size[0] - logo_hw - 15
    y_offset_header = (header_size[1] - logo_hh) // 2
    
    header_img.paste(logo_header, (x_offset_header, y_offset_header), logo_header)
    
    header_out_path = os.path.join(assets_dir, "installer_header.bmp")
    header_img.save(header_out_path, "BMP")
    print(f"[SUCCESS] Header banner saved to {header_out_path}")

if __name__ == "__main__":
    main()
