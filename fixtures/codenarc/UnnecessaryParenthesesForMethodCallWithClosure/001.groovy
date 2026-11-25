
            (1..10).inject(0) { acc, count ->
                if (delegate."msn$count"?.length() > 0) {
                    acc++
                }
                acc
            }
        