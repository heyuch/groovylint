
            def a = new Object() {
                String[] m1() {
                    def b = new Object() {
                        String[] m1() {
                            return null
                        }
                        String[] m2() {
                            return null
                        }
                    }
                    return [] as String[]
                }
            }
        