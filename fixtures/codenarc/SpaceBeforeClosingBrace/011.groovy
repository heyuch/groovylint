
            while (ready) { println 9}
            while (ready ||
                    done) { println 9}
            while (ready) println '{'  // no block; ignore
        