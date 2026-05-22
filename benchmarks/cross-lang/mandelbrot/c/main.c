#include <stdio.h>

int main(void) {
    const int width = 800;
    const int height = 800;
    const int max_iter = 100;

    for (int y = 0; y < height; y++) {
        for (int x = 0; x < width; x++) {
            double cx = ((double)x / width) * 3.5 - 2.5;
            double cy = ((double)y / height) * 2.0 - 1.0;
            double zx = 0.0, zy = 0.0;
            int i;
            for (i = 0; i < max_iter; i++) {
                double zx2 = zx * zx;
                double zy2 = zy * zy;
                if (zx2 + zy2 > 4.0) break;
                double new_zx = zx2 - zy2 + cx;
                double new_zy = 2.0 * zx * zy + cy;
                zx = new_zx;
                zy = new_zy;
            }
            putchar(i == max_iter ? '*' : ' ');
        }
        putchar('\n');
    }
    return 0;
}
