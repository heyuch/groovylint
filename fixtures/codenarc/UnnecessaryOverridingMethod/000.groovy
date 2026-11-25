
            class MyLabel extends javax.swing.JLabel {

                void setVisible(boolean value, int value2) {
                    super.setVisible(value)
                }

                void setVisible(String value) {
                    proxy.setVisible(value)
                }

                void setVisible(boolean value, String value2) {
                    super.setVisible(value2, value)
                }

                void someMethod(value) {
                    super.someMethod(value())
                }
            }
        