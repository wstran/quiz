import React, { useEffect, useState } from "react";
import axios from "axios";

const Route = () => {
    const handleGoogleLogin = () => {
        const width = 500;
        const height = 600;
        const left = (window.screen.width - width) / 2;
        const top = (window.screen.height - height) / 2;
        const popup = window.open(
            "https://cuda.network/auth/google",
            "Google Login",
            `width=${width},height=${height},top=${top},left=${left}`
        );

        if (!popup) {
            alert("Popup blocked! Please allow popups for this site.");
        }
    };

    useEffect(() => {
        const handleMessage = (event: MessageEvent) => {
            if (event.origin !== "https://cuda.network") return;

            const data: { status: string; token?: string } = event.data;

            if (data.status === "success") {
                window.location.href = "https://cuda.network";
            }
        };

        window.addEventListener("message", handleMessage);

        return () => window.removeEventListener("message", handleMessage);
    }, []);

    const connectWebSocket = async (room_code: string, nickname: string) => {
        const wsUrl = `wss://cuda.network/api/play?room_code=${room_code}&nickname=${nickname}`;

        const socket = new WebSocket(wsUrl);


        socket.onmessage = (event) => {
            console.log("Received:", JSON.parse(event.data));
        };

        socket.onerror = (error) => {
            console.error("WebSocket Error:", error);
        };

        socket.onclose = () => {
            console.log("WebSocket Disconnected");
        };
    };

    const [gamePin, setGamePin] = useState<string>('');
    const [nickname, setNickname] = useState<string>('');

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();

        if (gamePin.length !== 8) return alert('Game PIN must be 8 digits!');
        if (!nickname) return alert("Please enter a valid nickname");

        connectWebSocket(gamePin, nickname);
    };

    return (
        <div className="min-h-screen bg-gradient-to-br from-purple-600 via-blue-500 to-teal-400 flex items-center justify-center">
            <button
                onClick={handleGoogleLogin}
                className="absolute top-4 right-4 flex items-center gap-2 px-4 py-2 bg-white text-gray-700 font-semibold rounded-full shadow-md hover:bg-gray-100 transition-colors border border-gray-300"
            >
                <svg className="w-5 h-5" viewBox="0 0 24 24">
                    <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
                    <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-1.04.78-2.36 1.24-3.71 1.24-2.85 0-5.27-1.92-6.13-4.5H2.25v2.82C4.06 20.22 7.74 23 12 23z" />
                    <path fill="#FBBC05" d="M5.87 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.25C1.54 8.43 1 9.97 1 11.5s.54 3.07 1.25 4.43l3.62-2.84z" />
                    <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.74 1 4.06 3.78 2.25 7.07l3.62 2.84C6.73 7.33 9.15 5.38 12 5.38z" />
                </svg>
                Login with Google
            </button>
            <div className="relative bg-white rounded-lg shadow-2xl p-8 w-full max-w-md">
                <h1 className="text-4xl font-bold text-center text-purple-700 mb-6">
                    MI School!
                </h1>
                <p className="text-center text-gray-600 mb-8">
                    Enter game PIN and your nickname to join
                </p>
                <form onSubmit={handleSubmit} className="space-y-6">
                    <input
                        type="text"
                        value={nickname}
                        onChange={(e) => setNickname(e.target.value)}
                        maxLength={15}
                        placeholder="Your Nickname"
                        className="w-full p-4 text-center text-2xl font-semibold text-gray-800 border-2 border-purple-200 rounded-lg focus:outline-none focus:border-purple-700 transition-colors"
                    />
                    <input
                        type="text"
                        value={gamePin}
                        onChange={(e) => setGamePin(e.target.value.replace(/\D/g, ''))}
                        maxLength={8}
                        placeholder="Game PIN"
                        className="w-full p-4 text-center text-2xl font-semibold text-gray-800 border-2 border-purple-200 rounded-lg focus:outline-none focus:border-purple-700 transition-colors"
                    />
                    <button
                        type="submit"
                        className="w-full py-4 bg-yellow-400 text-white font-bold text-xl rounded-lg hover:bg-yellow-500 transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed"
                        disabled={gamePin.length !== 8 || nickname.trim().length === 0}
                    >
                        Join Game
                    </button>
                </form>
            </div>
        </div>
    );
}

export default Route;