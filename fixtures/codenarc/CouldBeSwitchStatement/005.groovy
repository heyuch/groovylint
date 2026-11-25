
        if (x == 1) {
            y = x
        } else if (x == 2) {
            y = x * 2
        } else if (x.find { it == 3}) {
            y = x * 3
        } else {
            y = 0
        }