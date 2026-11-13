/* Exercise the fault hook before building applications; touch only the supplied paths. */
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 3) return 1;
    return rename(argv[1], argv[2]) == 0 ? 0 : 1;
}
