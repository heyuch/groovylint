
            if (p.value instanceof Integer) {
                x = p.value * 2
            } else if (p.otherValue instanceof String) {
                x = p.otherValue * 2
            } else if (p.value instanceof Boolean) {
                x = !p.value
            }

             if (x.value instanceof Integer) {
                x = x.value * 2
            } else if (y.value instanceof String) {
                x = y.otherValue * 2
            } else if (z.value instanceof Boolean) {
                x = !z.value
            }
        