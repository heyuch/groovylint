
            long longMethod() {
                if (foo) return Math.random()
            }
            Long longMethod2() {
                return ((foo) ?: Math.random())
            }
        