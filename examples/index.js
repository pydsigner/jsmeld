import { logMessage } from './other.js';
import './index.css';

export class Greeter {
    constructor(name) {
        this.name = name;
    }

    greet() {
        return `Hello, ${this.name}!`;
    }
    sendMessage(message) {
        logMessage(message);
    }
}
