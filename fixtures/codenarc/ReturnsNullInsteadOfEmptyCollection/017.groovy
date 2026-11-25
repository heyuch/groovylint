
            def a = new Object() {
                List m1() {
                    def b = new Object() {
                        List m1() {
                            return null
                        }
                        List m2() {
                            return null
                        }
                    }
                    return []
                }
            }
        