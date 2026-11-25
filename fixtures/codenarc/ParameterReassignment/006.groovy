
            int myMethod(Integer a) {
                def comparable = new Comparable<Integer>() {
                    int compareTo(Integer s) {
                        s = null
                        return 0
                    }
                }
                return comparable.compareTo(a)
            }
        