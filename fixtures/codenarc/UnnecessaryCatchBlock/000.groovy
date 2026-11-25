
            try {
                foo()
            } catch (Exception e) {
                throw new RuntimeException(e)
            }
            try {
                foo()
            } catch (IOException e1) {
                throw e1
            } catch (Exception e2) {
                throw new RuntimeException(e2)
            }
        