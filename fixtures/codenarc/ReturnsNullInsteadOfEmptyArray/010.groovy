
            def a = new Object() {
                String[] m1() {
                    def b = new Object() {
                        void m1() {
                            def z = {
                                if (q) {
                                    return null
                                } else {
                                    return [] as String[]
                                }
                            }
                        }
                    }
                    return [] as String[]
                }
            }
        