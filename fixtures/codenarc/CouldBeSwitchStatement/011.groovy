
            if (x == 1) {
               y = x
            } else if (x == 2) {
               y = x * 2
            } else if (x == 3) {
               y = x * 3
            } else {
               y = 0
            }

            if (y instanceof Integer) {
               x = y + 1
            }
            if (y instanceof String) {
               x = y + '1'
            } else if (y instanceof Boolean) {
               x = !y
            } else {
               x = null
            }

            if (p.value instanceof Integer) {
                x = p.value * 2
            } else if (p.value instanceof String) {
                x = p.value + '1'
            } else if (p.value instanceof Boolean) {
                x = !p.value
            }
        