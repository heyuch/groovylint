
            import static Constants.CONSTANT1
            import static Constants.CONSTANT2
            import static Constants.CONSTANT3
            import static Constants.CONSTANT4
            import static Constants.CONSTANT5
            import static Constants.CONSTANT6
        
            void doSomething() {
                def stringA = "$CONSTANT1$CONSTANT2"
                def stringB = "$CONSTANT3${CONSTANT4}"
                def stringC = "${CONSTANT5}$CONSTANT6"
            }        
        