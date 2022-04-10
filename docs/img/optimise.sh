#/bin/bash

rm *.gif;

for f in *.webp;
do
	python3 -c "from PIL import Image;Image.open('$f').save('${f%.webp}.gif','gif',save_all=True,optimize=False,background=0,loop=0)";
done


for i in *.gif;
do
	ffmpeg -y -i "$i" -filter_complex "fps=5,scale=300:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=32[p];[s1][p]paletteuse=dither=bayer" ${i%.*}_.gif;
	rm ${i%.*}.gif;
	mv ${i%.*}_.gif ${i%.*}.gif;
done
